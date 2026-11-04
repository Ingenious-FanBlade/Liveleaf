import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createServer } from "vite";

const server = await createServer({ server: { middlewareMode: true }, appType: "custom" });
try {
  const { parseCourse } = await server.ssrLoadModule("/src/markdown/parseCourse.ts");
  const { readVisualization, VISUALIZATION_MIME } = await server.ssrLoadModule("/src/runtime/visualization.ts");
  const fixtures = JSON.parse(await readFile(new URL("./fixtures/visualization.json", import.meta.url), "utf8"));
  for (const [i, fixture] of fixtures.entries()) {
    const course = parseCourse(fixture.source);
    if (fixture.slides) assert.equal(course.slides.length, fixture.slides);
    if (fixture.static) { assert.equal(course.codeCells.length, 0); continue; }
    const cell = course.codeCells[0];
    assert.ok(cell, `fixture ${i}`);
    if (fixture.config) assert.deepEqual(cell.visualization, fixture.config, `fixture ${i}`);
    if (fixture.errors) assert.deepEqual(cell.visualizationDiagnostics?.map(d => d.attribute), fixture.errors, `fixture ${i}`);
    if (fixture.line) assert.equal(cell.sourceRange.startLine, fixture.line, `fixture ${i}`);
  }
  const config = { type: "tree", watch: "root", pointers: ["node"], scope: "module", tree: { valueField: "value", childFields: ["left", "right"] } };
  const trace = { schemaVersion: 1, runId: "test", cellId: "cell", config, frames: [{ seq: 0, type: "tree", event: "line", position: { cellLine: 1, functionName: "<module>", frameId: "f1", scopeFrameId: "f1" }, status: "ready", diagnosticCodes: [], state: { rootId: "o1", nodes: [{ id: "o1", value: "<script>alert(1)</script>", children: [{ field: "left", index: null, nodeId: null }, { field: "right", index: null, nodeId: null }] }] }, pointers: [{ name: "node", nodeId: "o1", status: "matched" }] }], capture: { status: "complete" }, diagnostics: [] };
  const read = value => readVisualization([{ kind: "json", mime: VISUALIZATION_MIME, value: JSON.stringify(value) }], "cell");
  assert.ok(read(trace).trace);
  assert.match(read({ ...trace, schemaVersion: 2 }).error, /schemaVersion/);
  const cyclic = structuredClone(trace); cyclic.frames[0].state.nodes[0].children[0].nodeId = "o1";
  assert.ok(read(cyclic).error);
  const missing = structuredClone(trace); missing.frames[0].pointers[0].nodeId = "missing";
  assert.ok(read(missing).error);
  const invalid = structuredClone(trace); invalid.frames[0].status = "invalid";
  assert.ok(read(invalid).error);

  const graph = structuredClone(trace);
  graph.config = { type: "graph", watch: "graph", pointers: ["node"], scope: "module", graph: { nodeStates: "states", edgeStates: "edge_states" }, tables: ["rows"] };
  graph.frames[0].type = "graph";
  graph.frames[0].state = {
    objectId: "graph1", directed: true,
    nodes: [{ id: "o1", value: "A", state: { distance: 0, visited: true, predecessor: null } }, { id: "o2", value: "B", state: { distance: 2, visited: false, predecessor: "A" } }, { id: "o3", value: "G", state: { distance: { kind: "float", text: "inf" } } }],
    edges: [{ id: "ab", source: "o1", target: "o2", weight: 2, state: { capacity: 8, flow: 2 } }, { id: "ba", source: "o2", target: "o1", weight: 3, state: {} }, { id: "aa", source: "o1", target: "o1", weight: 0, state: {} }],
    annotations: { nodeStates: "ready", edgeStates: "ready" },
  };
  graph.frames[0].tables = [{ name: "rows", status: "ready", state: { objectId: "rows1", columns: ["node", "distance"], rows: [["A", 0], ["B", 2]] } }];
  assert.ok(read(graph).trace, "Directed graph accepts cycles, self-loops and isolated nodes");
  const legacySnapshot = structuredClone(graph);
  legacySnapshot.config.graph = {};
  legacySnapshot.frames[0].state.annotations = { distances: "off", visited: "off", predecessors: "off" };
  assert.match(read(legacySnapshot).error, /协议版本不一致.*重新编译并重启桌面应用/, "An older desktop runtime must report a protocol mismatch instead of an invalid graph");
  const legacyConfig = structuredClone(graph);
  legacyConfig.config.graph = { distances: "distances", visited: "visited", predecessors: "previous" };
  assert.match(read(legacyConfig).error, /协议版本不一致/, "An older configuration needs the same actionable version diagnostic");
  const brokenGraph = change => { const t = structuredClone(graph); change(t.frames[0].state, t); assert.ok(read(t).error); };
  brokenGraph(s => { s.directed = false; });
  brokenGraph(s => { s.edges[0].target = "missing"; });
  brokenGraph(s => { s.nodes[1].id = "o1"; });
  brokenGraph(s => { s.edges.push({ ...s.edges[0], id: "duplicate" }); });
  brokenGraph(s => { s.edges[0].weight = true; });
  brokenGraph(s => { s.edges[0].weight = { kind: "float", text: "inf" }; });
  brokenGraph(s => { s.nodes[1].state.bad = []; });
  brokenGraph(s => { s.edges[0].state.bad = {}; });
  brokenGraph(s => { s.annotations.nodeStates = "unbound"; });
  brokenGraph(s => { s.annotations.edgeStates = "off"; });
  brokenGraph(s => { s.nodes[0].value = true; });
  brokenGraph((_s, t) => { t.config.graph.nodeStates = "states[0]"; });
  brokenGraph((_s, t) => { t.frames[0].tables[0].state.rows[0].pop(); });
  brokenGraph((_s, t) => { t.frames[0].tables[0].state.columns = ["same", "same"]; });
  brokenGraph((_s, t) => { t.frames[0].tables[0].status = "invalid"; });
  brokenGraph((_s, t) => { t.frames[0].tables[0].name = "other"; });
  brokenGraph((_s, t) => { delete t.frames[0].tables; });
  const table = structuredClone(graph);
  table.config = { type: "dynamic-table", watch: "rows", pointers: [], scope: "module" };
  table.frames[0] = { ...table.frames[0], type: "dynamic-table", state: table.frames[0].tables[0].state, pointers: [] };
  delete table.frames[0].tables;
  assert.ok(read(table).trace, "Dynamic tables run independently of graphs");
  const unavailableGraph = structuredClone(graph); unavailableGraph.frames[0].state = null; unavailableGraph.frames[0].status = "invalid"; unavailableGraph.frames[0].pointers[0] = { name: "node", status: "outside", nodeId: null };
  assert.ok(read(unavailableGraph).trace, "Tables remain available when the graph is invalid");
  const { layoutGraph } = await server.ssrLoadModule("/src/visualization/graphLayout.ts");
  const layout = layoutGraph(graph.frames[0].state);
  const relaxed = structuredClone(graph.frames[0].state); relaxed.nodes[1].state.distance = 1; relaxed.edges[0].state.flow = 3;
  assert.deepEqual(layoutGraph(relaxed), layout, "Relaxations must preserve graph geometry");
  assert.equal(new Set(layout.edges.map(e => e.path)).size, 3, "Reciprocal edges and self-loops have distinct paths");
  assert.equal(layoutGraph({ ...graph.frames[0].state, nodes: [], edges: [] }).nodes.length, 0);
  const example = parseCourse(await readFile(new URL("../docs/examples/dijkstra.ll.md", import.meta.url), "utf8"));
  assert.equal(example.codeCells.find(c => c.id === "dijkstra-shortest-path").visualization.type, "graph");
  assert.equal(example.codeCells.at(-1).visualization.type, "dynamic-table");
  assert.match(example.slides.find(s => s.html.includes('sl-syntax-function">dijkstra')).html, /sl-syntax-function">dijkstra<\/span>/);
  console.log(`Passed ${fixtures.length} shared syntax fixtures and frontend protocol checks.`);
} finally { await server.close(); }
