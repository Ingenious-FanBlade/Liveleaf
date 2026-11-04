import unittest
import test_visualization as helper

TABLE = {"type": "dynamic-table", "watch": "rows", "pointers": [], "scope": "module"}


class TableTests(unittest.TestCase):
    run_cell = helper.HookTests.run_cell
    ready = helper.HookTests.ready
    codes = helper.HookTests.codes

    def test_record_aliases_dynamic_columns_rows_and_history(self):
        code = "record={'node':'A','distance':4}\nrows=[record,{'node':'B'}]\nrecord['distance']=3\nrows[1]['visited']=True\nrows.append({'extra':2})\ndel rows[0]\ndel rows[0]['visited']\n"
        p, frames, records = self.run_cell(code, TABLE)
        self.assertEqual(p.returncode, 0, p.stderr)
        ready = self.ready(frames)
        self.assertEqual(ready[0]["state"]["columns"], ["node", "distance"])
        self.assertEqual(ready[0]["state"]["rows"], [["A", 4], ["B", None]])
        self.assertTrue(any(f["state"]["columns"] == ["node", "distance", "visited", "extra"] for f in ready))
        self.assertEqual(ready[-1]["state"]["columns"], ["node", "extra"])
        self.assertEqual(ready[-1]["state"]["rows"], [["B", None], [None, 2]])
        self.assertEqual(records[-1]["capture"]["status"], "complete")

    def test_vectors_ragged_matrices_empty_rows_and_rebinding(self):
        p, frames, _ = self.run_cell("rows=[]\nrows=[1,True,float('inf'),2**80]\nrows=[[1,2],(3,)]\nrows=[{}]\nrows=()\n", TABLE)
        self.assertEqual(p.returncode, 0, p.stderr)
        states = [f["state"] for f in self.ready(frames)]
        self.assertEqual(states[0]["rows"], [])
        self.assertEqual(states[1]["columns"], ["value"])
        self.assertEqual(states[1]["rows"][-1], [{"kind": "integer", "decimal": str(2**80)}])
        self.assertEqual(states[2]["columns"], ["0", "1"])
        self.assertEqual(states[2]["rows"], [[1,2],[3,None]])
        self.assertEqual(states[3]["rows"], [[]])
        self.assertEqual(states[-1]["columns"], [])
        self.assertEqual(len({s["objectId"] for s in states}), len(states))

    def test_multiple_tables_remain_independent_of_invalid_graph(self):
        config = {"type": "graph", "watch": "graph", "pointers": [], "scope": "module", "graph": {}, "tables": ["rows", "queue"]}
        p, frames, records = self.run_cell("rows=[{'cost':1}]\ngraph=None\nqueue='bad'\nqueue=[2,3]\nrows[0]['cost']=4\ngraph={}\n", config)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertTrue(any(f["status"] == "unbound" and f["tables"][0]["status"] == "ready" for f in frames))
        self.assertTrue(any(f["status"] == "invalid" and f["tables"][1]["status"] == "ready" for f in frames))
        self.assertEqual(frames[-1]["tables"][0]["state"]["rows"], [[4]])
        self.assertEqual(frames[-1]["tables"][1]["state"]["rows"], [[2],[3]])
        self.assertIn("InvalidDynamicTable", self.codes(records))

    def test_invalid_tables_recover_and_do_not_call_user_hooks(self):
        bad = ["None", "{}", "[{'x':[]} ]", "[{'':1}]", "[{1:2}]", "[{},[]]", "[[1],2]", "[Bad()]", "Rows([1])"]
        for value in bad:
            code = "calls=[]\nclass Bad:\n    def __str__(self):\n        calls.append('str')\n        return 'bad'\n    def __repr__(self):\n        calls.append('repr')\n        return 'bad'\nclass Rows(list): pass\nrows=" + value + "\nrows=[]\nprint(calls)\n"
            p, frames, records = self.run_cell(code, TABLE)
            self.assertEqual(p.returncode, 0, p.stderr)
            self.assertEqual(p.stdout.strip(), "[]")
            self.assertTrue(any(f["status"] == "invalid" for f in frames), value)
            self.assertEqual(frames[-1]["status"], "ready")
            self.assertEqual(records[-1]["capture"]["status"], "complete")

    def test_row_and_member_byte_budgets_continue_code(self):
        for code, reason in [("rows=list(range(257))\n", "tableRowLimit"), ("rows=[{str(i):'x'*256 for i in range(1100)}]\n", "recordLimit")]:
            p, _, records = self.run_cell(code + "print('continued')\n", TABLE)
            self.assertEqual(p.returncode, 0, p.stderr)
            self.assertEqual(p.stdout.strip(), "continued")
            self.assertEqual(records[-1]["capture"], {"status": "truncated", "reason": reason})


if __name__ == "__main__":
    unittest.main()
