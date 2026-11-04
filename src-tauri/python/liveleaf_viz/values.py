import math
import json
from decimal import Decimal

UNBOUND = object()


class Invalid(Exception):
    def __init__(self, code, message):
        self.code, self.message = code, message


class Limit(Exception):
    pass


def scalar(value, available=256 * 1024):
    t = type(value)
    if value is None or t is bool:
        return value
    if t is int:
        if abs(value) <= 9007199254740991:
            return value
        if value.bit_length() > max(0, available) * 4:
            raise Limit("recordLimit")
        # Decimal(int) is exact and avoids CPython's int-to-string digit limit
        # without changing a process-wide setting visible to user code.
        return {"kind": "integer", "decimal": str(Decimal(value))}
    if t is float:
        if math.isfinite(value):
            return value
        return {"kind": "float", "text": "nan" if math.isnan(value) else ("inf" if value > 0 else "-inf")}
    if t is str:
        if len(value) > 256:
            raise Invalid("ValueTooLarge", "Strings may contain at most 256 Unicode code points.")
        return value
    raise Invalid("UnsupportedValue", "Only exact built-in scalar types are supported.")


class Registry:
    def __init__(self):
        self.objects = {}
        self.graph_keys = {}
        self.value_bytes = 0

    def encode(self, value):
        encoded = scalar(value, 256 * 1024 - self.value_bytes)
        self.value_bytes += len(json.dumps(encoded, ensure_ascii=True, allow_nan=False))
        if self.value_bytes > 256 * 1024:
            raise Limit("recordLimit")
        return encoded

    def identify(self, value):
        key = id(value)
        existing = self.objects.get(key)
        if existing is not None and existing[0] is value:
            return existing[1]
        if len(self.objects) + len(self.graph_keys) >= 512:
            raise Limit("objectLimit")
        name = "o" + str(len(self.objects) + 1)
        self.objects[key] = (value, name)
        return name

    def identify_key(self, owner, key):
        typed = (owner, type(key), key)
        if typed not in self.graph_keys:
            if len(self.objects) + len(self.graph_keys) >= 512:
                raise Limit("objectLimit")
            self.graph_keys[typed] = "g" + str(len(self.graph_keys) + 1)
        return self.graph_keys[typed]

    def clear(self):
        self.objects.clear()
        self.graph_keys.clear()
