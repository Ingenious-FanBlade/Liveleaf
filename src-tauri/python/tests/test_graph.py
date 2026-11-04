import re
import unittest
from pathlib import Path

import test_visualization as helper

GRAPH = {"type": "graph", "watch": "graph", "pointers": ["current", "neighbor"], "scope": "module", "graph": {}}
DIJKSTRA = {**GRAPH, "scope": "dijkstra", "graph": {"nodeStates": "node_states"}, "tables": ["table_rows"]}
EXAMPLE = Path(__file__).resolve().parents[3] / "docs" / "examples" / "dijkstra.ll.md"


def example_code():
    return re.search(r'^```python[^\n]*id="dijkstra-shortest-path"[^\n]*\n(.*?)\n```', EXAMPLE.read_text(encoding="utf-8"), re.S | re.M).group(1) + "\n"


class GraphTests(unittest.TestCase):
    run_cell = helper.HookTests.run_cell
    ready = helper.HookTests.ready
    codes = helper.HookTests.codes

    def test_dijkstra_example_distances_relaxations_and_unreachable_node(self):
        p, frames, records = self.run_cell(example_code(), DIJKSTRA)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("到 F 的路径: A → C → B → D → E → F", p.stdout)
        self.assertIn("A → G: inf", p.stdout)
        self.assertEqual(records[-1]["capture"]["status"], "complete")
        self.assertLess(len(frames), 2000)
        ready = self.ready(frames)
        final = {n["value"]: n["state"] for n in ready[-1]["state"]["nodes"]}
        self.assertEqual({k: n["distance"] for k, n in final.items()}, {"A": 0, "B": 3, "C": 1, "D": 4, "E": 7, "F": 8, "G": {"kind": "float", "text": "inf"}})
        self.assertEqual({k for k, n in final.items() if n["visited"]}, {"A", "B", "C", "D", "E", "F"})
        self.assertEqual({k: n["predecessor"] for k, n in final.items()}, {"A": None, "B": "C", "C": "A", "D": "B", "E": "D", "F": "E", "G": None})
        for node, before, after in [("B", 4, 3), ("D", 6, 4), ("E", 10, 7)]:
            distances = [n["state"]["distance"] for f in ready for n in f["state"]["nodes"] if n["value"] == node and "distance" in n["state"]]
            self.assertIn(before, distances)
            self.assertIn(after, distances)
            self.assertLess(distances.index(before), distances.index(after))
        self.assertTrue(any(f["state"]["annotations"]["nodeStates"] == "unbound" for f in ready))
        self.assertTrue(any(f["pointers"][0]["status"] == "matched" and f["pointers"][1]["status"] == "matched" for f in ready))
        table = ready[-1]["tables"][0]["state"]
        self.assertEqual(table["columns"], ["node", "distance", "visited", "predecessor"])
        self.assertEqual(table["rows"][5], ["F", 8, True, "E"])
        for f in ready:
            t = f["tables"][0]
            if t["status"] == "ready":
                states = {n["value"]: n["state"] for n in f["state"]["nodes"]}
                for row in t["state"]["rows"]:
                    self.assertEqual(dict(zip(t["state"]["columns"], row)), states[row[0]])

    def test_cycles_self_edges_reciprocal_edges_and_isolated_nodes(self):
        code = "graph={'A':{'B':4},'B':{'A':2,'B':0},'C':{}}\ncurrent='A'\nneighbor='B'\n"
        p, frames, records = self.run_cell(code, GRAPH)
        self.assertEqual(p.returncode, 0, p.stderr)
        s = frames[-1]["state"]
        self.assertEqual(len(s["nodes"]), 3)
        self.assertEqual(len(s["edges"]), 3)
        self.assertEqual([p["status"] for p in frames[-1]["pointers"]], ["matched", "matched"])
        self.assertEqual(records[-1]["capture"]["status"], "complete")
        self.assertNotIn("NotATree", self.codes(records))

    def test_implicit_sinks_unweighted_empty_and_typed_keys(self):
        p, frames, _ = self.run_cell("graph={}\ngraph={1:['1'],'1':(2**80,)}\ncurrent=1\nneighbor='1'\n", GRAPH)
        self.assertEqual(p.returncode, 0, p.stderr)
        ready = self.ready(frames)
        self.assertEqual(ready[0]["state"]["nodes"], [])
        s = ready[-1]["state"]
        self.assertEqual(len(s["nodes"]), 3)
        self.assertTrue(all(e["weight"] is None for e in s["edges"]))
        self.assertNotEqual(ready[-1]["pointers"][0]["nodeId"], ready[-1]["pointers"][1]["nodeId"])
        self.assertEqual(s["nodes"][-1]["value"], {"kind": "integer", "decimal": str(2**80)})

    def test_reconnect_rebinding_and_independent_history(self):
        code = "graph={'A':{'B':1},'B':{}}\ncurrent=''.join(['B'])\ndel graph['A']['B']\ndel graph['B']\ngraph['B']={}\ngraph['A']['B']=2\ngraph={'A':{'B':2},'B':{}}\nneighbor=True\n"
        p, frames, _ = self.run_cell(code, GRAPH)
        self.assertEqual(p.returncode, 0, p.stderr)
        ready = self.ready(frames)
        first_id = ready[0]["state"]["objectId"]
        first_b = next(n["id"] for n in ready[0]["state"]["nodes"] if n["value"] == "B")
        old = [f for f in ready if f["state"]["objectId"] == first_id]
        self.assertTrue(any(f["pointers"][0]["status"] == "outside" for f in old))
        self.assertEqual({p["nodeId"] for f in old for p in f["pointers"] if p["status"] == "matched"}, {first_b})
        self.assertNotEqual(ready[-1]["state"]["objectId"], first_id)
        self.assertEqual(ready[0]["state"]["edges"][0]["weight"], 1)
        self.assertEqual(ready[-1]["state"]["edges"][0]["weight"], 2)
        self.assertEqual(ready[-1]["pointers"][1]["status"], "invalidType")

    def test_invalid_graphs_recover_without_stopping_program(self):
        samples = ["None", "{'A':{'B':True}}", "{'A':{'B':float('nan')}}", "{'A':['B','B']}", "{True:[]}", "{1.0:[]}", "{'A':object()}"]
        for value in samples:
            p, frames, records = self.run_cell("graph=" + value + "\ngraph={}\nprint('done')\n", GRAPH)
            self.assertEqual(p.returncode, 0, p.stderr)
            self.assertEqual(p.stdout.strip(), "done")
            self.assertTrue(any(f["status"] == "invalid" for f in frames))
            self.assertEqual(frames[-1]["status"], "ready")
            self.assertEqual(records[-1]["capture"]["status"], "complete")

    def test_invalid_auxiliary_values_only_disable_their_annotations(self):
        code = "graph={'A':{'B':1},'B':{}}\nnodes='bad'\nnodes={'A':{'bad':[]}}\nnodes={'outside':{}}\nnodes={'A':{'distance':0,'visited':True,'predecessor':None,'custom':'yes'}}\nedges={('B','A'):{'flow':2}}\nedges={('A','B'):{'flow':2,'capacity':8}}\n"
        config = {**GRAPH, "graph": {"nodeStates": "nodes", "edgeStates": "edges"}}
        p, frames, records = self.run_cell(code, config)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertTrue(all(f["status"] == "ready" for f in frames if f["status"] != "unbound"))
        self.assertEqual(frames[-1]["state"]["annotations"], {"nodeStates": "ready", "edgeStates": "ready"})
        self.assertEqual(frames[-1]["state"]["nodes"][0]["state"]["custom"], "yes")
        self.assertEqual(frames[-1]["state"]["edges"][0]["state"], {"flow": 2, "capacity": 8})
        self.assertIn("InvalidGraphNodeStates", self.codes(records))
        self.assertIn("InvalidGraphEdgeStates", self.codes(records))

    def test_arbitrary_members_partial_maps_and_independent_history(self):
        code = "graph={1:{'1':0},'1':{1:2}}\nnodes={1:{'field'+str(i):i for i in range(70)}}\nedges={(1,'1'):{'flow':2,'enabled':True}}\nnodes[1]['field0']=9\ndel nodes[1]['field1']\nedges[(1,'1')]['flow']=3\n"
        p, frames, records = self.run_cell(code, {**GRAPH, "graph": {"nodeStates": "nodes", "edgeStates": "edges"}})
        self.assertEqual(p.returncode, 0, p.stderr)
        ready = [f for f in frames if f["state"] and f["state"]["annotations"]["nodeStates"] == "ready"]
        self.assertEqual(len(ready[0]["state"]["nodes"][0]["state"]), 70)
        self.assertEqual(ready[0]["state"]["nodes"][0]["state"]["field0"], 0)
        self.assertNotIn("field1", ready[-1]["state"]["nodes"][0]["state"])
        self.assertEqual(ready[-1]["state"]["nodes"][1]["state"], {})
        self.assertEqual(ready[-1]["state"]["edges"][1]["state"], {})
        self.assertEqual(records[-1]["capture"]["status"], "complete")

    def test_graph_resource_budgets_continue_user_code(self):
        samples = [("graph={i:{} for i in range(129)}\n", "nodeLimit"), ("graph={i:{j:1 for j in range(128)} for i in range(5)}\n", "edgeLimit"), ("graph={}\nfor i in range(600):\n    graph={i:{}}\n", "objectLimit")]
        for code, reason in samples:
            p, _, records = self.run_cell(code + "print('continued')\n", GRAPH)
            self.assertEqual(p.returncode, 0, p.stderr)
            self.assertEqual(p.stdout.strip(), "continued")
            self.assertEqual(records[-1]["capture"], {"status": "truncated", "reason": reason})

    def test_unsupported_objects_do_not_invoke_hooks(self):
        code = "calls=[]\nclass Bad:\n    def __str__(self):\n        calls.append('str')\n        return 'x'\n    def __repr__(self):\n        calls.append('repr')\n        return 'x'\ngraph={'A':{'B':Bad()}}\ncurrent=Bad()\ngraph={'A':{}}\nprint(calls)\n"
        p, frames, _ = self.run_cell(code, GRAPH)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(p.stdout.strip(), "[]")
        self.assertEqual(frames[-1]["pointers"][0]["status"], "invalidType")

    def test_dijkstra_rejects_negative_weights_and_preserves_exception_trace(self):
        p, frames, records = self.run_cell(example_code().replace('"B": 4', '"B": -4'), DIJKSTRA)
        self.assertNotEqual(p.returncode, 0)
        self.assertIn("ValueError", p.stderr)
        self.assertTrue(self.ready(frames))
        self.assertEqual(records[-1]["capture"]["status"], "complete")

    def test_dijkstra_handles_implicit_sinks_zero_weights_and_mixed_key_ties(self):
        function = example_code().split("\ngraph = {", 1)[0]
        code = function + "\ngraph={1:{'sink':0,2:0}}\ndijkstra(graph,1)\n"
        p, frames, _ = self.run_cell(code, DIJKSTRA)
        self.assertEqual(p.returncode, 0, p.stderr)
        nodes = frames[-1]["state"]["nodes"]
        self.assertEqual({n["value"]: n["state"]["distance"] for n in nodes}, {1: 0, "sink": 0, 2: 0})
        self.assertTrue(all(n["state"]["visited"] for n in nodes))


if __name__ == "__main__":
    unittest.main()
