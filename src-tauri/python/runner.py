"""Execute an unchanged cell as __main__, with an optional bounded observer."""
import ast
import json
import keyword
import os
import re
import sys
import types
from liveleaf_viz.collector import Collector


class VisualizationConfigError(Exception):
    pass


def validate(config, source, filename):
    def name(value):
        return type(value) is str and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", value) and not keyword.iskeyword(value)

    def names(values, empty=False):
        return type(values) is list and (empty or len(values) > 0) and all(name(v) for v in values) and len(set(values)) == len(values)

    if type(config) is not dict or any(k not in ("type", "watch", "scope", "pointers", "tree", "graph", "tables") for k in config) or config.get("type") not in ("array", "tree", "graph", "dynamic-table") or not name(config.get("watch")) or not name(config.get("scope")) or not names(config.get("pointers"), True) or len(config["pointers"]) > 8:
        raise VisualizationConfigError("Invalid visualization configuration.")
    if "tables" in config and not names(config["tables"], True):
        raise VisualizationConfigError("Invalid dynamic table bindings.")
    if config["type"] == "dynamic-table" and config["pointers"]:
        raise VisualizationConfigError("dynamic-table does not use pointers.")
    if config["type"] == "tree":
        tree = config.get("tree", {})
        if not name(tree.get("valueField")) or not names(tree.get("childFields")):
            raise VisualizationConfigError("Invalid tree fields.")
    elif "tree" in config:
        raise VisualizationConfigError("Only tree accepts tree fields.")
    if config["type"] == "graph":
        graph = config.get("graph")
        if type(graph) is not dict or any(k not in ("nodeStates", "edgeStates") or not name(v) for k, v in graph.items()):
            raise VisualizationConfigError("Invalid graph observation variables.")
    elif "graph" in config:
        raise VisualizationConfigError("Only graph accepts graph observation variables.")
    syntax = ast.parse(source, filename)
    for node in ast.walk(syntax):
        if isinstance(node, (ast.AsyncFunctionDef, ast.Await, ast.Yield, ast.YieldFrom, ast.GeneratorExp)) or isinstance(node, ast.comprehension) and node.is_async:
            raise VisualizationConfigError("Generators and asynchronous control flow are unsupported for visualization.")
    code = compile(source, filename, "exec")
    scope_code = None
    if config["scope"] != "module":
        definitions = [n for n in syntax.body if isinstance(n, ast.FunctionDef) and n.name == config["scope"]]
        if len(definitions) != 1 or definitions[0].decorator_list:
            raise VisualizationConfigError("scope must name one undecorated top-level synchronous function.")
        matches = [c for c in code.co_consts if isinstance(c, types.CodeType) and c.co_name == config["scope"] and c.co_firstlineno == definitions[0].lineno]
        if len(matches) != 1:
            raise VisualizationConfigError("Ambiguous function scope.")
        scope_code = matches[0]
    return code, scope_code


def main():
    filename, config_path, trace_path = map(os.path.abspath, sys.argv[1:4])
    with open(config_path, encoding="utf-8") as file:
        header = json.load(file)
    with open(filename, encoding="utf-8") as file:
        source = file.read()
    code, scope_code = validate(header["config"], source, filename)
    module = types.ModuleType("__main__")
    module.__file__, module.__package__, module.__spec__ = filename, None, None
    sys.modules["__main__"] = module
    sys.argv = [filename]
    sys.path[0] = os.path.dirname(filename)
    collector = None
    get_trace, set_trace = getattr(sys, "gettrace", None), getattr(sys, "settrace", None)
    previous = get_trace() if callable(get_trace) else None
    try:
        try:
            collector = Collector(trace_path, header, filename, scope_code)
            if sys.implementation.name != "cpython" or sys.version_info < (3, 10) or not callable(get_trace) or not callable(set_trace):
                collector.diagnostic("UnsupportedInterpreter", "Visualization requires CPython 3.10 or later.")
                collector.finish("error", "unsupportedInterpreter")
            else:
                set_trace(collector.callback)
        except BaseException:
            if collector:
                try:
                    collector.finish("error", "collectorSetupError")
                except BaseException:
                    pass
        exec(code, module.__dict__)
    finally:
        try:
            if collector:
                try:
                    collector.close()
                except BaseException:
                    pass
        finally:
            if callable(set_trace):
                try:
                    set_trace(previous)
                except BaseException:
                    pass


if __name__ == "__main__":
    main()
