import assert from "node:assert/strict";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { createSSRApp, h } from "vue";
import { renderToString } from "vue/server-renderer";
import { parse, compileStyle } from "vue/compiler-sfc";
import { createServer } from "vite";

const server = await createServer({ server: { middlewareMode: true }, appType: "custom" });
try {
  const load = async name => (await server.ssrLoadModule(`/src/visualization/${name}.vue`)).default;
  const GraphView = await load("GraphView"), TreeView = await load("TreeView"), ArrayView = await load("ArrayView");
  const DynamicTableView = await load("DynamicTableView"), TracePlayer = await load("TracePlayer");
  const render = (component, props) => renderToString(createSSRApp({ render: () => h(component, props) }));
  const svgOf = html => html.match(/<svg\b[\s\S]*?<\/svg>/)[0];
  const textsOf = svg => [...svg.matchAll(/<text\b[^>]*>(.*?)<\/text>/g)].map(m => m[1]);
  const pointer = (name, nodeId) => ({ name, nodeId, status: "matched" });
  const graph = {
    objectId: "graph", directed: true,
    nodes: [
      { id: "a", value: "A", state: { distance: 0, visited: true, predecessor: null } },
      { id: "b", value: "B", state: { distance: 3, visited: true, predecessor: "A", custom: "<script>unsafe</script>" } },
      { id: "c", value: "C", state: { distance: { kind: "float", text: "inf" }, visited: false } },
    ],
    edges: [
      { id: "ab", source: "a", target: "b", weight: 3, state: { flow: 3, capacity: 8, enabled: true } },
      { id: "ba", source: "b", target: "a", weight: 2, state: {} },
      { id: "bb", source: "b", target: "b", weight: 0, state: {} },
      { id: "bc", source: "b", target: "c", weight: 5, state: {} },
    ],
    annotations: { nodeStates: "ready", edgeStates: "ready" },
  };
  const previous = structuredClone(graph); previous.nodes[1].state.distance = 4; previous.edges[0].state.flow = 0;
  const graphHtml = await render(GraphView, { state: graph, previous, pointers: [pointer("current", "b"), pointer("neighbor", "c")] });
  const graphSvg = svgOf(graphHtml);
  assert.deepEqual(textsOf(graphSvg), ["3", "2", "0", "5", "A", "B", "C"], "Graph canvas shows only identifiers and weights");
  assert.doesNotMatch(graphSvg, /<marker\b|marker-end|stroke-dasharray/, "No detached markers or gaps at arrow joins");
  assert.equal((graphSvg.match(/class="edge-arrow"/g) ?? []).length, graph.edges.length, "Each arrow is a single filled path");
  assert.doesNotMatch(graphSvg, /class="edge-shaft"|class="edge-head"/);
  assert.match(graphSvg, /class="(?=[^"]*\bgraph-node\b)(?=[^"]*\bupdated\b)[^"]*"/, "Any node state change is highlighted");
  assert.match(graphSvg, /class="(?=[^"]*\bgraph-edge\b)(?=[^"]*\bupdated\b)[^"]*"/, "Any edge state change is highlighted");
  assert.doesNotMatch(graphSvg, /class="[^"]*\bvisited\b/, "Member names have no built-in algorithm semantics");
  assert.match(graphSvg, /class="reference-ring primary"/);
  assert.match(graphSvg, /class="reference-ring secondary"/);
  assert.match(graphSvg, /<title\b[^>]*>B · current · distance: 3 · visited: True · predecessor: &quot;A&quot;/, "Hover preserves arbitrary node members without diagram labels");
  assert.match(graphHtml, /<dt[^>]*>capacity<\/dt><dd[^>]*>8<\/dd>/, "Arbitrary edge members appear in details");
  assert.doesNotMatch(graphHtml, /<table\b|<script>unsafe/, "Graph has no embedded table, and member values are escaped");
  assert.match(graphHtml, /<details[^>]*class="viz-details"[^>]*>/);
  assert.doesNotMatch(graphHtml, /<details[^>]*\bopen(?:[\s=>])/, "Observer details start collapsed");
  const coincident = svgOf(await render(GraphView, { state: graph, pointers: [pointer("current", "b"), pointer("neighbor", "b")] }));
  assert.match(coincident, /class="reference-ring primary" r="39"/);
  assert.match(coincident, /class="reference-ring secondary" r="44"/, "Coincident references stay distinguishable");

  const table = { objectId: "rows", columns: ["node", "distance", "note"], rows: [["A", 0, null], ["B", 3, "<script>unsafe</script>"]] };
  const oldTable = { objectId: "rows", columns: ["distance", "node"], rows: [[0, "A"], [4, "B"]] };
  const tableHtml = await render(DynamicTableView, { state: table, previous: oldTable, name: "table_rows" });
  assert.match(tableHtml, /动态表格/);
  assert.match(tableHtml, /class="dynamic-table-view viz-state-view"/, "Standalone tables receive the shared change highlight colors");
  assert.equal((tableHtml.match(/class="changed"/g) ?? []).length, 3, "Cell comparisons follow column names across column reordering");
  assert.doesNotMatch(tableHtml, /<script>/);
  assert.match(tableHtml, /&lt;script&gt;unsafe&lt;\/script&gt;/);
  assert.doesNotMatch(await render(DynamicTableView, { state: { ...table, objectId: "new" }, previous: oldTable }), /class="changed"/, "Rebinding resets table comparisons");
  const emptyTableHtml = await render(DynamicTableView, { state: { objectId: "empty", columns: [], rows: [] } });
  assert.match(emptyTableHtml, /空表格/);
  const invalidGraphFrame = { seq: 0, type: "graph", status: "invalid", state: null, pointers: [], diagnosticCodes: [], event: "line", position: { cellLine: 1, functionName: "<module>", frameId: "f1", scopeFrameId: "f1" }, tables: [{ name: "rows", status: "ready", state: table }] };
  const playerHtml = await render(TracePlayer, { trace: { runId: "run", config: { type: "graph", watch: "graph", pointers: [], scope: "module", graph: {}, tables: ["rows"] }, frames: [invalidGraphFrame], capture: { status: "complete" }, diagnostics: [] }, source: "graph=None", runStatus: "success" });
  assert.match(playerHtml, /当前主体不可用/);
  assert.match(playerHtml, /class="dynamic-table-view viz-state-view"/, "Independent table renders even when the primary graph is invalid");

  const { layoutGraph, graphArrowOutline } = await server.ssrLoadModule("/src/visualization/graphLayout.ts");
  const layout = layoutGraph(graph);
  assert.equal(new Set(layout.edges.map(e => e.path)).size, graph.edges.length);
  const near = (a, b) => assert.ok(Math.abs(a - b) < 1e-7, `${a} ≈ ${b}`);
  const distance = (a, b) => Math.hypot(a.x - b.x, a.y - b.y);
  for (const edge of layout.edges) {
    const source = graph.edges.find(e => e.id === edge.id);
    const target = layout.nodes.find(n => n.id === source.target);
    const head = { x: edge.tip.x - edge.base.x, y: edge.tip.y - edge.base.y };
    const tangent = { x: edge.base.x - edge.control.x, y: edge.base.y - edge.control.y };
    near(head.x * tangent.y - head.y * tangent.x, 0);
    assert.ok(head.x * tangent.x + head.y * tangent.y > 0, "Shaft and head face the same way, including self-loops");
    near((edge.left.x + edge.right.x) / 2, edge.base.x);
    near((edge.left.y + edge.right.y) / 2, edge.base.y);
    assert.ok(distance(edge.left, edge.right) / 2 > 2, "Even the widest highlighted shaft fits inside the arrowhead");
    assert.ok(distance(edge.tip, target) > 45.5, "Arrow clears the node and both reference rings");
    for (const width of [2, 3.5, 4]) {
      const outline = graphArrowOutline(edge, width);
      assert.equal((outline.match(/M /g) ?? []).length, 1, "No independent arrowhead subpath");
      assert.ok(outline.endsWith(" Z"), "Shaft and head form a closed contour");
      assert.ok(!/NaN|Infinity/.test(outline));
    }
  }

  const leaf = (id, value) => ({ id, value, children: [{ field: "left", index: null, nodeId: null }, { field: "right", index: null, nodeId: null }] });
  const tree = { rootId: "root", nodes: [
    { ...leaf("root", "A"), children: [{ field: "left", index: null, nodeId: "left" }, { field: "right", index: null, nodeId: "right" }] },
    leaf("left", "B"),
    { ...leaf("right", "C"), children: [{ field: "left", index: null, nodeId: null }, { field: "right", index: null, nodeId: "last" }] },
    leaf("last", "F"),
  ] };
  const treeHtml = await render(TreeView, { state: tree, pointers: [pointer("node", "right"), pointer("parent", "right")] });
  const treeSvg = svgOf(treeHtml);
  assert.deepEqual(textsOf(treeSvg), ["&quot;B&quot;", "&quot;F&quot;", "&quot;C&quot;", "&quot;A&quot;"], "Tree has no per-edge or per-node status labels");
  assert.match(treeSvg, /<title\b[^>]*>left<\/title>/, "Child field names remain available on hover");
  assert.match(treeSvg, /class="reference-ring secondary"[^>]*width="110"/, "Tree preserves overlapping references");
  const treeNodes = [...treeSvg.matchAll(/transform="translate\(([\d.]+), ([\d.]+)\)"[^>]*>[\s\S]*?<text[^>]*>(.*?)<\/text>/g)];
  const xOf = name => Number(treeNodes.find(n => n[3] === `&quot;${name}&quot;`)[1]);
  assert.ok(xOf("F") > xOf("C"), "A lone right child stays to the right without requiring a text label");

  const arrayProps = { state: { objectId: "array", items: [1, 2, 3] }, previous: { objectId: "array", items: [1, 0, 3] }, pointers: [
    { name: "i", raw: 1, index: 1, status: "inBounds" },
    { name: "j", raw: 1, index: 1, status: "inBounds" },
  ] };
  const arrayHtml = await render(ArrayView, arrayProps);
  const arrayStrip = arrayHtml.match(/<div class="array-strip"[\s\S]*?<div class="state-legend"/)[0];
  assert.match(arrayStrip, /class="(?=[^"]*\barray-item\b)(?=[^"]*\bchanged\b)(?=[^"]*\bprimary\b)(?=[^"]*\bsecondary\b)[^"]*"/, "Changed and overlapping references coexist in arrays");
  assert.doesNotMatch(arrayStrip.replace(/title="[^"]*"/g, ""), /已变化|pointer-label|i=|j=/);
  assert.match(arrayStrip, />\[1\]<\/small>/, "Array indices remain visible");

  // Optional local artifact uses the actual SSR markup and compiled component styles.
  if (process.argv.includes("--preview")) {
    let css = await readFile(new URL("../src/visualization/visualization.css", import.meta.url), "utf8");
    for (const name of ["GraphView", "TreeView", "ArrayView", "StateLegend", "DynamicTableView"]) {
      const component = await load(name);
      const source = await readFile(new URL(`../src/visualization/${name}.vue`, import.meta.url), "utf8");
      for (const style of parse(source).descriptor.styles) css += compileStyle({ source: style.content, id: component.__scopeId, scoped: !!style.scoped }).code;
    }
    const html = `<!doctype html><meta charset="utf-8"><title>Visualization review</title><style>
      body{background:#fbf7ea;color:#263747;font:16px system-ui;margin:28px}main{display:grid;grid-template-columns:600px 600px;gap:24px;align-items:start}section{padding:20px;border:3px solid #334454;border-radius:16px;box-shadow:5px 5px #334454}h1{font-size:24px}h2{font-size:20px;margin-top:0}${css}
      </style><h1>可视化检查</h1><main><section><h2>有向图 · 节点与边状态</h2>${graphHtml}${tableHtml}</section><section><h2>二叉树 · 右孩子位置与重合引用</h2>${treeHtml}<h2>数组 · 变化与重合引用</h2>${arrayHtml}</section></main>`;
    await mkdir(new URL("../dist/", import.meta.url), { recursive: true });
    await writeFile(new URL("../dist/visualization-review.html", import.meta.url), html);
    console.log("Preview saved to dist/visualization-review.html");
  }
  console.log("Passed visualization rendering and arrow geometry checks (array, tree, directed graph, dynamic table).");
} finally { await server.close(); }
