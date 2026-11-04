"""Dynamic tables observe arrays independently of any graph or algorithm."""
from .values import Invalid, Limit


def column(value, registry):
    if type(value) is not str or not value or len(value) > 256:
        raise Invalid("InvalidTableColumn", "Table column names must be non-empty strings of at most 256 characters.")
    return registry.encode(value)


def snapshot(value, registry):
    if type(value) is not list and type(value) is not tuple:
        raise Invalid("InvalidDynamicTable", "A dynamic table must reference an exact list or tuple.")
    if len(value) > 256:
        raise Limit("tableRowLimit")
    object_id = registry.identify(value)
    columns, rows = [], []
    if value:
        first = type(value[0])
        if first is dict:
            seen = set()
            for row in value:
                if type(row) is not dict:
                    raise Invalid("InvalidDynamicTable", "Table rows must all be records, arrays, or scalars.")
                for name in row:
                    column(name, registry)
                    if name not in seen:
                        seen.add(name)
                        columns.append(name)
            get = lambda row, i: row.get(columns[i])
        elif first is list or first is tuple:
            if any(type(row) is not list and type(row) is not tuple for row in value):
                raise Invalid("InvalidDynamicTable", "Table rows must all be records, arrays, or scalars.")
            columns = [column(str(i), registry) for i in range(max(map(len, value)))]
            get = lambda row, i: row[i] if i < len(row) else None
        else:
            columns = ["value"]
            get = lambda row, i: row
        # Every output cell costs at least one JSON byte; avoid expanding sparse
        # records into an unbounded intermediate matrix before the byte check.
        if len(value) * len(columns) > 256 * 1024 - registry.value_bytes:
            raise Limit("recordLimit")
        rows = [[registry.encode(get(row, i)) for i in range(len(columns))] for row in value]
    return {"objectId": object_id, "columns": columns, "rows": rows}


def pointers(names, lookup, state, registry):
    return []
