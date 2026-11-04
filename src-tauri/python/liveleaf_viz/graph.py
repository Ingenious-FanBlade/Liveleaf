"""Bounded directed adjacency-map snapshots, with optional algorithm annotations."""
import math
from .values import UNBOUND, Invalid, Limit

ANNOTATIONS = ("nodeStates", "edgeStates")


def key(value):
    if type(value) is not str and type(value) is not int:
        raise Invalid("InvalidGraphKey", "Graph node keys must be exact built-in strings or integers.")
    if type(value) is str and len(value) > 256:
        raise Invalid("ValueTooLarge", "Node keys may contain at most 256 Unicode code points.")
    return value


def number(value, finite):
    return type(value) is int or type(value) is float and (math.isfinite(value) if finite else not math.isnan(value))


def snapshot(value, registry):
    if type(value) is not dict:
        raise Invalid("InvalidGraph", "watch must reference an exact built-in adjacency dictionary.")
    if len(value) > 128:
        raise Limit("nodeLimit")
    object_id = registry.identify(value)
    nodes, edges, ids = [], [], {}

    def add_node(node):
        node = key(node)
        typed = (type(node), node)
        if typed not in ids:
            if len(nodes) >= 128:
                raise Limit("nodeLimit")
            node_id = registry.identify_key(object_id, node)
            ids[typed] = node_id
            nodes.append({"id": node_id, "value": registry.encode(node), "state": {}})
        return ids[typed]

    # Validate every key before any dictionary lookup can encounter user key hooks.
    for node in value:
        add_node(node)
    for node, adjacent in value.items():
        source = ids[(type(node), node)]
        if type(adjacent) is dict:
            weighted = True
        elif type(adjacent) is list or type(adjacent) is tuple:
            weighted = False
        else:
            raise Invalid("InvalidAdjacency", "Neighbors must be a weight dictionary or an exact list/tuple of node keys.")
        if len(edges) + len(adjacent) > 512:
            raise Limit("edgeLimit")
        seen = set()
        pairs = adjacent.items() if weighted else ((child, None) for child in adjacent)
        for child, weight in pairs:
            target = add_node(child)
            if target in seen:
                raise Invalid("DuplicateEdge", "Parallel edges are not supported in an adjacency list.")
            seen.add(target)
            if weighted and not number(weight, finite=True):
                raise Invalid("InvalidEdgeWeight", "Edge weights must be finite exact built-in int/float values; bool is not a weight.")
            edges.append({"id": "e-" + source + "-" + target, "source": source, "target": target, "weight": registry.encode(weight), "state": {}})
    return {"objectId": object_id, "directed": True, "nodes": nodes, "edges": edges, "annotations": {name: "off" for name in ANNOTATIONS}}


def members(value, registry):
    if type(value) is not dict:
        raise Invalid("InvalidGraphState", "Each node/edge state must be an exact dictionary of named scalar members.")
    result = {}
    for name, item in value.items():
        if type(name) is not str or not name or len(name) > 256:
            raise Invalid("InvalidGraphState", "State member names must be non-empty strings of at most 256 characters.")
        registry.encode(name)
        result[name] = registry.encode(item)
    return result


def annotate(state, config, lookup, registry):
    nodes = {n["id"]: n for n in state["nodes"]}
    # Read original keys from the bounded registry without decoding scalar tags.
    keys = {(kind, value): node_id for (owner, kind, value), node_id in registry.graph_keys.items() if owner == state["objectId"] and node_id in nodes}
    edges = {(e["source"], e["target"]): e for e in state["edges"]}
    diagnostics = []

    def node_id(value):
        key(value)
        result = keys.get((type(value), value))
        if result is None:
            raise Invalid("UnknownGraphNode", "An annotation refers to a node outside the current graph.")
        return result

    for name in ANNOTATIONS:
        variable = config.get(name)
        if variable is None:
            continue
        value = lookup(variable)
        if value is UNBOUND:
            state["annotations"][name] = "unbound"
            continue
        try:
            if type(value) is not dict:
                raise Invalid("InvalidGraphState", "Node and edge states must be exact dictionaries.")
            if len(value) > (len(nodes) if name == "nodeStates" else len(edges)):
                raise Invalid("InvalidGraphState", "State entries must refer to existing nodes or directed edges.")
            updates = []
            for target, item in value.items():
                if name == "nodeStates":
                    entry = nodes[node_id(target)]
                else:
                    if type(target) is not tuple or len(target) != 2:
                        raise Invalid("InvalidGraphState", "Edge state keys must be exact (source, target) tuples.")
                    pair = (node_id(target[0]), node_id(target[1]))
                    if pair not in edges:
                        raise Invalid("InvalidGraphState", "An edge state refers to a nonexistent directed edge.")
                    entry = edges[pair]
                updates.append((entry, members(item, registry)))
            for entry, item in updates:
                entry["state"] = item
            state["annotations"][name] = "ready"
        except Invalid as error:
            state["annotations"][name] = "invalid"
            diagnostics.append(("InvalidGraph" + name[0].upper() + name[1:], variable + ": " + error.message))
    return diagnostics


def pointers(names, lookup, state, registry):
    reachable = {n["id"] for n in state["nodes"]} if state else set()
    result = []
    for name in names:
        value, node_id = lookup(name), None
        if value is UNBOUND:
            status = "unbound"
        elif value is None:
            status = "null"
        else:
            try:
                key(value)
                candidate = registry.graph_keys.get((state["objectId"], type(value), value)) if state else None
                status = "matched" if candidate in reachable else "outside"
                if status == "matched":
                    node_id = candidate
            except Invalid:
                status = "invalidType"
        result.append({"name": name, "nodeId": node_id, "status": status})
    return result
