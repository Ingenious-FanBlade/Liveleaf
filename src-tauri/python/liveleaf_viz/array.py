from .values import UNBOUND, Invalid, Limit


def snapshot(value, registry):
    if type(value) is not list and type(value) is not tuple:
        raise Invalid("InvalidArray", "watch must reference an exact built-in list or tuple.")
    if len(value) > 256:
        raise Limit("arrayLimit")
    return {"objectId": registry.identify(value), "items": [registry.encode(item) for item in value]}


def pointers(names, lookup, state, registry):
    result = []
    for name in names:
        value = lookup(name)
        raw, index = None, None
        if value is UNBOUND:
            status = "unbound"
        elif type(value) is not int:
            status = "invalidType"
            try:
                raw = registry.encode(value)
            except Invalid:
                pass
        else:
            raw = registry.encode(value)
            length = len(state["items"]) if state else 0
            if state and -length <= value < length:
                index, status = value % length, "inBounds"
            else:
                status = "outOfBounds"
        result.append({"name": name, "raw": raw, "index": index, "status": status})
    return result
