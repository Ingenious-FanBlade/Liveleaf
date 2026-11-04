import json
import sys
from . import array, tree, graph, table
from .values import UNBOUND, Invalid, Limit, Registry

MAX_BYTES = 8 * 1024 * 1024
MAX_RECORD = 256 * 1024


class Collector:
    def __init__(self, path, header, filename, scope_code):
        self.file = open(path, "w", encoding="utf-8", newline="\n")
        self.config = header["config"]
        self.filename, self.scope_code = filename, scope_code
        self.registry = Registry()
        self.frames, self.bytes, self.frame_counter = 0, 0, 0
        self.frame_ids = {}
        self.diagnostics = set()
        self.closed = False
        self.entered = self.bound = False
        self.callback = self.trace
        self.write({"kind": "header", **header})

    def write(self, record, terminal=False):
        data = (json.dumps(record, ensure_ascii=True, allow_nan=False, separators=(",", ":")) + "\n").encode("utf-8")
        if not terminal:
            if len(data) > MAX_RECORD:
                raise Limit("recordLimit")
            if self.bytes + len(data) > MAX_BYTES - 16384:
                raise Limit("byteLimit")
        self.file.write(data.decode("utf-8"))
        self.file.flush()
        self.bytes += len(data)

    def diagnostic(self, code, message, seq=None):
        key = (code, message)
        if key in self.diagnostics or self.closed:
            return
        if len(self.diagnostics) >= 31:
            code, message = "DiagnosticsLimited", "Additional diagnostics were omitted."
            key = (code, message)
            if key in self.diagnostics:
                return
        self.diagnostics.add(key)
        self.write({"kind": "diagnostic", "diagnostic": {"code": code, "message": message, **({"seq": seq} if seq is not None else {})}})

    def finish(self, status="complete", reason=None):
        if self.closed:
            return
        try:
            self.write({"kind": "captureEnd", "capture": {"status": status, **({"reason": reason} if reason else {})}}, terminal=True)
        finally:
            self.closed = True
            self.file.close()
            self.registry.clear()
            self.frame_ids.clear()

    def identify_frame(self, frame):
        key = id(frame)
        if key not in self.frame_ids:
            self.frame_counter += 1
            self.frame_ids[key] = "f" + str(self.frame_counter)
        return self.frame_ids[key]

    def scope(self, frame):
        while frame is not None:
            if frame.f_code.co_filename == self.filename:
                if self.config["scope"] == "module" and frame.f_code.co_name == "<module>":
                    return frame
                if self.scope_code is not None and frame.f_code is self.scope_code:
                    return frame
            frame = frame.f_back
        return None

    def sample(self, frame, event):
        scope = self.scope(frame)
        if scope is None:
            return
        self.entered = True
        self.registry.value_bytes = 0
        if self.frames >= 2000:
            raise Limit("frameLimit")
        local_names = scope.f_code.co_varnames + scope.f_code.co_cellvars + scope.f_code.co_freevars

        def lookup(name):
            if name in scope.f_locals:
                return scope.f_locals[name]
            if name in local_names:
                return UNBOUND
            return scope.f_globals.get(name, UNBOUND)

        value = lookup(self.config["watch"])
        state, status, codes = None, "ready", []
        adapter = {"array": array, "tree": tree, "graph": graph, "dynamic-table": table}[self.config["type"]]
        if value is UNBOUND:
            status = "unbound"
        else:
            self.bound = True
            try:
                state = adapter.snapshot(value, self.config["tree"], self.registry) if adapter is tree else adapter.snapshot(value, self.registry)
                if adapter is graph:
                    for code, message in graph.annotate(state, self.config["graph"], lookup, self.registry):
                        codes.append(code)
                        self.diagnostic(code, message, self.frames)
            except Invalid as error:
                status, codes = "invalid", [error.code]
                self.diagnostic(error.code, error.message, self.frames)
        pointers = adapter.pointers(self.config["pointers"], lookup, state, self.registry)
        record = {"seq": self.frames, "event": event, "position": {"cellLine": max(1, frame.f_lineno), "functionName": frame.f_code.co_name, "frameId": self.identify_frame(frame), "scopeFrameId": self.identify_frame(scope)}, "status": status, "diagnosticCodes": codes, "type": self.config["type"], "state": state, "pointers": pointers}
        if "tables" in self.config:
            record["tables"] = []
            for name in self.config["tables"]:
                value = lookup(name)
                observed = {"name": name, "status": "unbound", "state": None}
                if value is not UNBOUND:
                    try:
                        observed.update(status="ready", state=table.snapshot(value, self.registry))
                    except Invalid as error:
                        observed["status"] = "invalid"
                        if error.code not in codes and len(codes) < 32:
                            codes.append(error.code)
                        self.diagnostic(error.code, name + ": " + error.message, self.frames)
                record["tables"].append(observed)
        self.write({"kind": "frame", "frame": record})
        self.frames += 1

    def trace(self, frame, event, arg):
        if self.closed or frame.f_code.co_filename != self.filename:
            return None
        if event not in ("call", "line", "return", "exception"):
            return self.callback
        try:
            self.sample(frame, event)
        except Limit as error:
            try:
                self.finish("truncated", error.args[0])
            except BaseException:
                self.closed = True
        except BaseException:
            # An observer failure must never replace the user's exception or stop code.
            try:
                self.diagnostic("CollectorError", "Snapshot collection failed; user code continued.")
                self.finish("error", "collectorError")
            except BaseException:
                self.closed = True
        finally:
            if event == "return":
                self.frame_ids.pop(id(frame), None)
        return None if self.closed else self.callback

    def close(self):
        try:
            if not self.closed:
                if sys.gettrace() is not self.callback:
                    self.diagnostic("TraceModified", "User code replaced or disabled the tracing hook.")
                    self.finish("error", "TraceModified")
                else:
                    if not self.entered:
                        self.diagnostic("ScopeNotEntered", "The selected function was never called.")
                    elif not self.bound:
                        self.diagnostic("WatchNeverBound", "The watched variable was never bound.")
                    self.finish()
        finally:
            self.registry.clear()
            self.frame_ids.clear()
            self.file.close()
