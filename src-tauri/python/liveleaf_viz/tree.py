import types
from collections import deque
from .values import UNBOUND, Invalid, Limit

TYPE_DICT = type.__dict__["__dict__"]
TYPE_MRO = type.__dict__["__mro__"]


def class_field(cls, name):
    for base in TYPE_MRO.__get__(cls):
        fields = TYPE_DICT.__get__(base)
        if name in fields:
            return fields[name]
    return UNBOUND


def is_node(value):
    if type(value) is dict:
        return True
    # Read type metadata through type's implementation, bypassing metaclass hooks.
    t = type(value)
    bases = TYPE_MRO.__get__(t)
    return all(base is not type for base in bases) and type(class_field(t, "__dict__")) is types.GetSetDescriptorType


def field(node, name):
    if type(node) is dict:
        if name not in node:
            raise Invalid("MissingField", "Missing node field: " + name)
        return node[name]
    if not is_node(node):
        raise Invalid("InvalidNode", "Nodes must be dictionaries or objects with ordinary instance fields.")
    cls = type(node)
    class_value = class_field(cls, name)
    descriptor_type = type(class_value)
    descriptor = class_field(descriptor_type, "__get__") is not UNBOUND
    if descriptor and (class_field(descriptor_type, "__set__") is not UNBOUND or class_field(descriptor_type, "__delete__") is not UNBOUND):
        raise Invalid("UnsupportedDescriptor", "Cannot observe descriptors, properties or slots: " + name)
    # Invoke only the exact built-in instance-dictionary descriptor. In particular,
    # object.__getattribute__(node, '__dict__') could invoke a user property.
    instance_fields = class_field(cls, "__dict__").__get__(node, cls)
    value = dict.get(instance_fields, name, UNBOUND)
    if value is not UNBOUND:
        return value
    value = class_value
    if value is UNBOUND:
        raise Invalid("MissingField", "Missing node field: " + name)
    if descriptor:
        raise Invalid("UnsupportedDescriptor", "Cannot observe descriptors, properties or slots: " + name)
    return value


def snapshot(root, config, registry):
    if root is None:
        return {"rootId": None, "nodes": []}
    nodes, seen, edges = [], set(), 0
    scheduled = {id(root)}
    queue = deque([(root, 0)])
    while queue:
        node, depth = queue.popleft()
        if id(node) in seen:
            raise Invalid("NotATree", "A node is referenced by multiple child positions or a cycle.")
        if not is_node(node):
            raise Invalid("InvalidNode", "A child is not a supported node.")
        seen.add(id(node))
        if len(seen) > 128:
            raise Limit("nodeLimit")
        if depth > 32:
            raise Limit("depthLimit")
        item = {"id": registry.identify(node), "value": registry.encode(field(node, config["valueField"])), "children": []}
        for name in config["childFields"]:
            child = field(node, name)
            multiple = type(child) is list or type(child) is tuple
            count = len(child) if multiple else 1
            if edges + count > 512:
                raise Limit("edgeLimit")
            edges += count
            for index in range(count):
                value = child[index] if multiple else child
                if value is not None and not is_node(value):
                    raise Invalid("InvalidNode", "A child is not a supported node.")
                item["children"].append({"field": name, "index": index if multiple else None, "nodeId": registry.identify(value) if value is not None else None})
                if value is not None:
                    if id(value) in scheduled:
                        raise Invalid("NotATree", "A node is referenced by multiple child positions or a cycle.")
                    if len(scheduled) >= 128:
                        raise Limit("nodeLimit")
                    scheduled.add(id(value))
                    queue.append((value, depth + 1))
        nodes.append(item)
    return {"rootId": registry.identify(root), "nodes": nodes}


def pointers(names, lookup, state, registry):
    reachable = {n["id"] for n in state["nodes"]} if state else set()
    result = []
    for name in names:
        value, node_id = lookup(name), None
        if value is UNBOUND:
            status = "unbound"
        elif value is None:
            status = "null"
        elif is_node(value):
            key = registry.identify(value)
            status = "matched" if key in reachable else "outside"
            if status == "matched":
                node_id = key
        else:
            status = "invalidType"
        result.append({"name": name, "nodeId": node_id, "status": status})
    return result
