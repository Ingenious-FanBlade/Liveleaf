import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

RUNNER = Path(__file__).resolve().parents[1] / "runner.py"
ARRAY = {"type": "array", "watch": "arr", "pointers": [], "scope": "module"}
TREE = {"type": "tree", "watch": "root", "pointers": ["node"], "scope": "module", "tree": {"valueField": "val", "childFields": ["left", "right"]}}
NODE = """class Node:
    def __init__(self, val, left=None, right=None):
        self.val, self.left, self.right = val, left, right
"""


class HookTests(unittest.TestCase):
    def run_cell(self, code, config=None, files=None):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parent) as directory:
            directory = Path(directory)
            (directory / "cell.py").write_text(code, encoding="utf-8")
            for name, content in (files or {}).items():
                (directory / name).write_text(content, encoding="utf-8")
            header = {"schemaVersion": 1, "runId": "test-run", "cellId": "test-cell", "config": config or ARRAY}
            (directory / "config.json").write_text(json.dumps(header), encoding="utf-8")
            process = subprocess.run([sys.executable, "-u", str(RUNNER), str(directory / "cell.py"), str(directory / "config.json"), str(directory / "trace.jsonl")], capture_output=True, text=True, encoding="utf-8", timeout=10, cwd=directory, env={**os.environ, "PYTHONIOENCODING": "utf-8"})
            path = directory / "trace.jsonl"
            records = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()] if path.exists() else []
            frames = [r["frame"] for r in records if r["kind"] == "frame"]
            self.assertEqual([f["seq"] for f in frames], list(range(len(frames))))
            self.assertTrue(all(len(json.dumps(r).encode()) <= 256 * 1024 for r in records))
            self.assertLessEqual(path.stat().st_size if path.exists() else 0, 8 * 1024 * 1024)
            return process, frames, records

    def ready(self, frames):
        return [f for f in frames if f["status"] == "ready"]

    def codes(self, records):
        return [r["diagnostic"]["code"] for r in records if r["kind"] == "diagnostic"]

    def test_stairs_final_assignment_and_line_semantics(self):
        p, frames, records = self.run_cell("n=5\narr=[0]*(n+1)\narr[0]=arr[1]=1\nfor i in range(2,n+1):\n    arr[i]=arr[i-1]+arr[i-2]\nprint(arr[n])\n", {**ARRAY, "pointers": ["i"]})
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(p.stdout.strip(), "8")
        self.assertEqual(frames[0]["status"], "unbound")
        self.assertEqual(frames[-1]["state"]["items"], [1, 1, 2, 3, 5, 8])
        before = next(f for f in frames if f["position"]["cellLine"] == 3)
        self.assertEqual(before["state"]["items"], [0] * 6)
        self.assertEqual(records[-1]["capture"]["status"], "complete")

    def test_snapshots_alias_and_rebinding_identity(self):
        p, frames, _ = self.run_cell("arr=[2,1]\nalias=arr\nalias[0]=3\narr.append(4)\ndel arr[1]\narr=[3,4]\narr[0]=9\n")
        ready = self.ready(frames)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(ready[0]["state"]["items"], [2, 1])
        self.assertEqual(ready[-1]["state"]["items"], [9, 4])
        same_content = [f["state"]["objectId"] for f in ready if f["state"]["items"] == [3, 4]]
        self.assertEqual(len(set(same_content)), 2)

    def test_pointer_negative_bounds_bool_and_unbound(self):
        _, frames, _ = self.run_cell("arr=[1,2]\ni=-1\nj=2\nb=True\nx='bad'\n", {**ARRAY, "pointers": ["i", "j", "b", "x", "missing"]})
        pointers = frames[-1]["pointers"]
        self.assertEqual([(p["status"], p["index"]) for p in pointers], [("inBounds", 1), ("outOfBounds", None), ("invalidType", None), ("invalidType", None), ("unbound", None)])
        self.assertEqual(pointers[0]["raw"], -1)

    def test_scalar_encoding_and_invalid_recovery(self):
        p, frames, records = self.run_cell("arr=[2**80,float('nan'),float('inf'),float('-inf'),True,None,'<script>']\narr=[object()]\narr=[]\narr=['x'*257]\narr=(1,)\n")
        ready = self.ready(frames)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(ready[0]["state"]["items"][0], {"kind": "integer", "decimal": str(2**80)})
        self.assertEqual(ready[0]["state"]["items"][1], {"kind": "float", "text": "nan"})
        self.assertIn("ValueTooLarge", self.codes(records))
        self.assertTrue(any(f["status"] == "invalid" for f in frames))
        self.assertEqual(ready[-1]["state"]["items"], [1])

    def test_exact_container_types_only(self):
        p, frames, _ = self.run_cell("class List(list): pass\narr=List([1])\narr=[[1]]\narr=None\narr=[]\n")
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(self.ready(frames)[-1]["state"]["items"], [])
        self.assertGreaterEqual(sum(f["status"] == "invalid" for f in frames), 3)

    def test_large_integer_preserves_digits_without_changing_python_settings(self):
        p, frames, _ = self.run_cell("import sys\nlimit=sys.get_int_max_str_digits() if hasattr(sys,'get_int_max_str_digits') else None\narr=[10**5000]\nassert limit == (sys.get_int_max_str_digits() if hasattr(sys,'get_int_max_str_digits') else None)\n")
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(frames[-1]["state"]["items"][0], {"kind": "integer", "decimal": "1" + "0"*5000})

    def test_tree_preorder_recursive_scope(self):
        code = NODE + "root=Node('A',Node('B',Node('D'),Node('E')),Node('C',None,Node('F')))\nvisited=[]\ndef preorder(node):\n    if node is None: return\n    visited.append(node.val)\n    preorder(node.left)\n    preorder(node.right)\npreorder(root)\nprint(' '.join(visited))\n"
        p, frames, _ = self.run_cell(code, {**TREE, "scope": "preorder"})
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(p.stdout.strip(), "A B D E C F")
        ready = self.ready(frames)
        self.assertEqual(len({f["position"]["scopeFrameId"] for f in ready}), 13)
        self.assertTrue(any(f["pointers"][0]["status"] == "null" for f in ready))
        state = ready[-1]["state"]
        c = next(n for n in state["nodes"] if n["value"] == "C")
        self.assertIsNone(c["children"][0]["nodeId"])
        self.assertIsNotNone(c["children"][1]["nodeId"])

    def test_tree_stable_ids_and_detached_pointers(self):
        p, frames, _ = self.run_cell(NODE + "root=Node(3,Node(3))\nnode=root.left\nroot.left=None\nroot.left=node\nnode.val=4\n", TREE)
        self.assertEqual(p.returncode, 0, p.stderr)
        ready = self.ready(frames)
        self.assertTrue(any(f["pointers"][0]["status"] == "outside" for f in ready))
        ids = {f["pointers"][0]["nodeId"] for f in ready if f["pointers"][0]["status"] == "matched"}
        self.assertEqual(len(ids), 1)
        self.assertEqual(len(ready[-1]["state"]["nodes"]), 2)

    def test_cycles_shared_children_recover(self):
        p, frames, records = self.run_cell(NODE + "root=Node(1)\nroot.left=root\nroot.left=None\nnode=Node(2)\nroot.left=root.right=node\nroot.right=None\nroot.left=[node]*128\nroot.left=node\n", TREE)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("NotATree", self.codes(records))
        self.assertEqual(frames[-1]["status"], "ready")
        self.assertTrue(any(f["status"] == "invalid" for f in frames))
        self.assertEqual(records[-1]["capture"]["status"], "complete")

    def test_dictionary_children_keep_empty_slots(self):
        c = {**TREE, "tree": {"valueField": "name", "childFields": ["children"]}}
        p, frames, _ = self.run_cell("root={'name':'A','children':[None,{'name':'B','children':[]}]}\nnode=root['children'][1]\n", c)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual([e["index"] for e in frames[-1]["state"]["nodes"][0]["children"]], [0, 1])

    def test_properties_and_conversion_hooks_are_never_called(self):
        code = """calls=[]
class Bad:
    @property
    def val(self):
        calls.append('property')
        return 1
    def __getattr__(self, name):
        calls.append('getattr')
        return None
    def __str__(self):
        calls.append('str')
        return 'bad'
    def __repr__(self):
        calls.append('repr')
        return 'bad'
    def __init__(self): self.left=self.right=None
root=Bad()
node=root
print(calls)
"""
        p, _, records = self.run_cell(code, TREE)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(p.stdout.strip(), "[]")
        self.assertIn("UnsupportedDescriptor", self.codes(records))

    def test_slots_are_explicitly_unsupported(self):
        p, frames, records = self.run_cell("class N:\n    __slots__=('val','left','right')\nroot=N()\nroot.val=1\nroot.left=root.right=None\n", TREE)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertFalse(self.ready(frames))
        self.assertIn("InvalidNode", self.codes(records))

    def test_local_unbound_does_not_fall_back_to_global(self):
        p, frames, _ = self.run_cell("arr=[99]\ndef f():\n    pass\n    arr=[1]\nf()\n", {**ARRAY, "scope": "f"})
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(frames[0]["status"], "unbound")
        self.assertEqual(frames[-1]["state"]["items"], [1])

    def test_helper_uses_selected_scope_and_module(self):
        code = "def helper(a):\n    a.append(2)\ndef f():\n    arr=[1]\n    helper(arr)\nf()\n"
        p, frames, _ = self.run_cell(code, {**ARRAY, "scope": "f"})
        self.assertEqual(p.returncode, 0, p.stderr)
        h = next(f for f in frames if f["position"]["functionName"] == "helper" and f["event"] == "return")
        self.assertNotEqual(h["position"]["frameId"], h["position"]["scopeFrameId"])
        self.assertEqual(h["state"]["items"], [1, 2])

    def test_scope_never_called_and_watch_never_bound(self):
        _, frames, records = self.run_cell("def f(): pass\n", {**ARRAY, "scope": "f"})
        self.assertEqual(frames, [])
        self.assertIn("ScopeNotEntered", self.codes(records))
        _, _, records = self.run_cell("print('ok')\n")
        self.assertIn("WatchNeverBound", self.codes(records))

    def test_invalid_scope_and_unsupported_control_flow_reject_before_execution(self):
        for code, scope in [("def f(): pass\ndef f(): pass", "f"), ("class X:\n    def f(self): pass", "f"), ("def gen():\n    yield 1", "module"), ("arr=(i for i in range(5))", "module"), ("async def f(): pass", "module"), ("def wrapper(f): return f\n@wrapper\ndef f(): pass", "f")]:
            p, frames, _ = self.run_cell("print('MUST NOT RUN')\n" + code + "\n", {**ARRAY, "scope": scope})
            self.assertNotEqual(p.returncode, 0)
            self.assertEqual(p.stdout, "")
            self.assertEqual(frames, [])
            self.assertIn("VisualizationConfigError", p.stderr)

    def test_exception_caught_and_uncaught(self):
        p, frames, records = self.run_cell("arr=[1]\ntry:\n    raise ValueError('caught')\nexcept ValueError:\n    arr.append(2)\nraise RuntimeError('failed')\n")
        self.assertNotEqual(p.returncode, 0)
        self.assertIn("RuntimeError: failed", p.stderr)
        self.assertTrue(any(f["event"] == "exception" for f in frames))
        self.assertEqual(frames[-1]["state"]["items"], [1, 2])
        self.assertEqual(records[-1]["capture"]["status"], "complete")

    def test_frame_limit_does_not_stop_user_code(self):
        p, frames, records = self.run_cell("arr=[]\nfor i in range(3000):\n    pass\nprint('finished')\n")
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(p.stdout.strip(), "finished")
        self.assertEqual(len(frames), 2000)
        self.assertEqual(records[-1]["capture"], {"status": "truncated", "reason": "frameLimit"})

    def test_array_object_tree_limits(self):
        samples = [("arr=list(range(257))\n", ARRAY, "arrayLimit"), ("arr=[]\nfor i in range(600):\n    arr=[]\n", ARRAY, "objectLimit"), (NODE + "root=Node(0)\nfor i in range(34):\n    root=Node(i,root)\n", TREE, "depthLimit"), ("root={'val':1,'left':[None]*513,'right':None}\n", TREE, "edgeLimit")]
        for code, config, reason in samples:
            p, _, records = self.run_cell(code + "print('done')\n", config)
            self.assertEqual(p.returncode, 0, p.stderr)
            self.assertEqual(p.stdout.strip(), "done")
            self.assertEqual(records[-1]["capture"], {"status": "truncated", "reason": reason})

    def test_trace_disabled_is_not_complete(self):
        p, _, records = self.run_cell("import sys\narr=[]\nsys.settrace(None)\narr.append(1)\n")
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("TraceModified", self.codes(records))
        self.assertEqual(records[-1]["capture"]["status"], "error")

    def test_byte_record_and_node_limits_keep_program_running(self):
        samples = [("arr=['a'*256]*256\nfor i in range(100):\n    pass\n", ARRAY, "byteLimit"), ("arr=['\\U0001f600'*256]*256\n", ARRAY, "recordLimit"), ("root={'val':0,'left':[{'val':i,'left':None,'right':None} for i in range(129)],'right':None}\n", TREE, "nodeLimit")]
        for code, config, reason in samples:
            p, _, records = self.run_cell(code + "print('done')\n", config)
            self.assertEqual(p.returncode, 0, p.stderr)
            self.assertEqual(p.stdout.strip(), "done")
            self.assertEqual(records[-1]["capture"], {"status": "truncated", "reason": reason})

    def test_observation_never_reads_custom_class_attribute(self):
        code = NODE + "class N(Node):\n    def __getattribute__(self, name):\n        if name == '__class__': raise AssertionError('hook called')\n        return super().__getattribute__(name)\nroot=N(1)\nnode=root\n"
        p, frames, records = self.run_cell(code, TREE)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(frames[-1]["status"], "ready")
        self.assertNotIn("CollectorError", self.codes(records))

    def test_metaclass_metadata_descriptors_are_not_executed(self):
        code = NODE + "calls=[]\nclass Meta(type):\n    @property\n    def __dict__(cls):\n        calls.append('dict')\n        return {}\n    def __eq__(cls, other):\n        calls.append('eq')\n        return False\nclass N(Node, metaclass=Meta): pass\nroot=N(1)\nnode=root\nprint(calls)\n"
        p, frames, records = self.run_cell(code, TREE)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(p.stdout.strip(), "[]")
        self.assertEqual(frames[-1]["status"], "ready")
        self.assertNotIn("CollectorError", self.codes(records))

    def test_internal_collector_failure_does_not_stop_user_code(self):
        p, _, records = self.run_cell("from liveleaf_viz.collector import Collector\narr=[]\nCollector.sample=lambda *args: 1/0\nprint('continued')\n")
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(p.stdout.strip(), "continued")
        self.assertIn("CollectorError", self.codes(records))
        self.assertEqual(records[-1]["capture"]["status"], "error")

    def test_future_import_dataclass_main_and_local_import(self):
        code = "from __future__ import annotations\nfrom dataclasses import dataclass\nimport __main__, sys, helper\n@dataclass\nclass N:\n    value: int\narr=[helper.VALUE]\ntrace='user'\nconfig='user'\nassert __main__.__dict__ is globals()\nassert sys.argv == [__file__]\nassert sys.path[0] == __import__('os').path.dirname(__file__)\nprint(N(1).value,trace,config)\n"
        p, frames, _ = self.run_cell(code, files={"helper.py": "VALUE=7"})
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(p.stdout.strip(), "1 user user")
        self.assertEqual(frames[-1]["state"]["items"], [7])
        self.assertFalse(any(f["position"]["functionName"] == "__repr__" for f in frames))

    def test_system_exit_and_syntax_error_keep_original_failure(self):
        p, _, records = self.run_cell("arr=[1]\nraise SystemExit(3)\n")
        self.assertEqual(p.returncode, 3)
        self.assertEqual(records[-1]["capture"]["status"], "complete")
        p, frames, _ = self.run_cell("arr=\n")
        self.assertNotEqual(p.returncode, 0)
        self.assertIn("SyntaxError", p.stderr)
        self.assertEqual(frames, [])


if __name__ == "__main__":
    unittest.main()
