<script setup lang="ts">
import { computed, ref } from "vue";
import type { GraphNode, GraphEdge, GraphMembers, GraphState } from "../runtime/graphVisualization";
import type { TreePointer, VizScalar } from "../runtime/visualization";
import { formatScalar } from "../runtime/visualization";
import { GRAPH_NODE_RADIUS, graphArrowOutline, layoutGraph } from "./graphLayout";
import StateLegend from "./StateLegend.vue";
import "./visualization.css";

const props = defineProps<{ state: GraphState; pointers: TreePointer[]; previous?: GraphState }>();
const zoom = ref(1);
const viewport = ref<HTMLElement>();
let drag: { x: number; y: number; left: number; top: number } | undefined;
const layout = computed(() => layoutGraph(props.state));
const nodes = computed(() => new Map(props.state.nodes.map(n => [n.id, n])));
const edges = computed(() => new Map(props.state.edges.map(e => [e.id, e])));
const oldNodes = computed(() => new Map(props.previous?.nodes.map(n => [n.id, n])));
const oldEdges = computed(() => new Map(props.previous?.edges.map(e => [e.id, e])));
const namesAt = (id: string) => props.pointers.filter(p => p.status === "matched" && p.nodeId === id).map(p => p.name).join(", ");
const primaryAt = (id: string) => props.pointers[0]?.status === "matched" && props.pointers[0].nodeId === id;
const secondaryAt = (id: string) => props.pointers.slice(1).some(p => p.status === "matched" && p.nodeId === id);
const changed = (a: GraphMembers, b?: GraphMembers) => !!b && (Object.keys(a).length !== Object.keys(b).length || Object.keys(a).some(k => !Object.prototype.hasOwnProperty.call(b, k) || JSON.stringify(a[k]) !== JSON.stringify(b[k])));
const updated = (n: GraphNode) => changed(n.state, oldNodes.value.get(n.id)?.state);
const edgeUpdated = (e: GraphEdge) => {
  const old = oldEdges.value.get(e.id);
  return !!old && (changed(e.state, old.state) || JSON.stringify(e.weight) !== JSON.stringify(old.weight));
};
const label = (value: VizScalar): string => typeof value === "string" ? value : formatScalar(value);
const describe = (members: GraphMembers) => Object.entries(members).map(([k, v]) => `${k}: ${formatScalar(v)}`).join(" · ");
const short = (text: string, max = 12) => Array.from(text).length > max ? Array.from(text).slice(0, max - 1).join("") + "…" : text;
function edgeKind(id: string) {
  const e = edges.value.get(id)!;
  if (props.pointers[0]?.nodeId === e.source && props.pointers[1]?.nodeId === e.target) return "current";
  return edgeUpdated(e) ? "updated" : "normal";
}
const drawnEdges = computed(() => layout.value.edges.map(edge => {
  const kind = edgeKind(edge.id);
    return { ...edge, kind, outline: graphArrowOutline(edge, kind === "normal" ? 2 : 4) };
}));
function start(event: PointerEvent) {
  if (!viewport.value || event.button !== 0) return;
  drag = { x: event.clientX, y: event.clientY, left: viewport.value.scrollLeft, top: viewport.value.scrollTop };
  viewport.value.setPointerCapture(event.pointerId);
}
function move(event: PointerEvent) {
  if (!drag || !viewport.value) return;
  viewport.value.scrollLeft = drag.left + drag.x - event.clientX;
  viewport.value.scrollTop = drag.top + drag.y - event.clientY;
}
const annotationNames = { nodeStates: "节点状态", edgeStates: "边状态" };
const annotationStatuses = { off: "未观察", ready: "可用", unbound: "尚未绑定", invalid: "当前不可用" };
</script>

<template>
  <div class="graph-view viz-state-view">
    <div class="graph-toolbar" aria-label="有向图缩放">
      <button type="button" :disabled="zoom <= .25" aria-label="缩小有向图" @click="zoom = Math.max(.25, zoom - .25)">−</button>
      <button type="button" aria-label="重置有向图缩放" @click="zoom = 1">{{ Math.round(zoom * 100) }}%</button>
      <button type="button" :disabled="zoom >= 2" aria-label="放大有向图" @click="zoom = Math.min(2, zoom + .25)">+</button>
      <small>{{ state.nodes.length }} 个节点 · {{ state.edges.length }} 条有向边 · 拖动画布或滚动查看</small>
    </div>
    <p v-if="!state.nodes.length">空有向图 · {}</p>
    <div v-else ref="viewport" class="graph-viewport" tabindex="0" aria-label="带权有向图，支持滚动和平移" @pointerdown="start" @pointermove="move" @pointerup="drag = undefined" @pointercancel="drag = undefined" @lostpointercapture="drag = undefined">
      <svg :width="layout.size * zoom" :height="layout.size * zoom" :viewBox="`0 0 ${layout.size} ${layout.size}`" role="img" aria-label="箭头表示边的方向，连边标签表示权重">
        <g v-for="e in drawnEdges" :key="e.id" class="graph-edge" :class="e.kind">
          <title>{{ label(nodes.get(edges.get(e.id)!.source)!.value) }} → {{ label(nodes.get(edges.get(e.id)!.target)!.value) }}{{ edges.get(e.id)!.weight !== null ? ` · ${formatScalar(edges.get(e.id)!.weight)}` : '' }}{{ describe(edges.get(e.id)!.state) ? ` · ${describe(edges.get(e.id)!.state)}` : '' }}{{ edgeUpdated(edges.get(e.id)!) ? ' · 状态变化' : '' }}</title>
          <path class="edge-arrow" :d="e.outline" />
          <text v-if="edges.get(e.id)!.weight !== null" :x="e.x" :y="e.y" text-anchor="middle">{{ short(formatScalar(edges.get(e.id)!.weight)) }}</text>
        </g>
        <g v-for="p in layout.nodes" :key="p.id" :transform="`translate(${p.x},${p.y})`" class="graph-node" :class="{ updated: updated(nodes.get(p.id)!) }">
          <title>{{ label(nodes.get(p.id)!.value) }}{{ namesAt(p.id) ? ` · ${namesAt(p.id)}` : '' }}{{ describe(nodes.get(p.id)!.state) ? ` · ${describe(nodes.get(p.id)!.state)}` : '' }}{{ updated(nodes.get(p.id)!) ? ' · 状态变化' : '' }}</title>
          <circle class="node-body" :r="GRAPH_NODE_RADIUS" />
          <circle v-if="primaryAt(p.id)" class="reference-ring primary" r="39" />
          <circle v-if="secondaryAt(p.id)" class="reference-ring secondary" :r="primaryAt(p.id) ? 44 : 39" />
          <text text-anchor="middle" dominant-baseline="middle">{{ short(label(nodes.get(p.id)!.value), 6) }}</text>
        </g>
      </svg>
    </div>
    <StateLegend :references="pointers.length" />
    <details v-if="pointers.length || Object.values(state.annotations).some(s => s !== 'off')" class="viz-details">
      <summary>观察详情</summary>
      <p v-for="p in pointers" :key="p.name">{{ p.name }}：{{ p.nodeId ? label(nodes.get(p.nodeId)!.value) : { null: '空引用', outside: '图外节点', unbound: '尚未绑定', invalidType: '无效类型', matched: '' }[p.status] }}</p>
      <p v-for="(status, name) in state.annotations" v-show="status !== 'off'" :key="name">{{ annotationNames[name] }}：{{ annotationStatuses[status] }}</p>
      <div class="graph-state-list">
        <div v-for="n in state.nodes.filter(n => Object.keys(n.state).length)" :key="n.id" class="graph-state-entry" :class="{ updated: updated(n) }">
          <strong>节点 {{ label(n.value) }}</strong>
          <dl><template v-for="(v, k) in n.state" :key="k"><dt>{{ k }}</dt><dd>{{ formatScalar(v) }}</dd></template></dl>
        </div>
        <div v-for="e in state.edges.filter(e => Object.keys(e.state).length)" :key="e.id" class="graph-state-entry" :class="{ updated: edgeUpdated(e) }">
          <strong>边 {{ label(nodes.get(e.source)!.value) }} → {{ label(nodes.get(e.target)!.value) }}</strong>
          <dl><template v-for="(v, k) in e.state" :key="k"><dt>{{ k }}</dt><dd>{{ formatScalar(v) }}</dd></template></dl>
        </div>
      </div>
    </details>
  </div>
</template>

<style scoped>
.graph-view { min-width: 0; }
.graph-toolbar { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin: 8px 0; }
.graph-toolbar button { min-height: 32px; min-width: 34px; padding: 3px 8px; border: 2px solid #334454; border-radius: 6px; background: #fffdf5; color: #263747; cursor: pointer; }
.graph-toolbar button:disabled { opacity: .45; cursor: default; }
.graph-toolbar small { font-size: .88rem; line-height: 1.5; }
.graph-viewport { max-width: 100%; max-height: 560px; overflow: auto; cursor: grab; touch-action: none; background-image: radial-gradient(#33445418 1px, transparent 1px); background-size: 20px 20px; }
.graph-viewport:active { cursor: grabbing; }
svg { display: block; max-width: none; user-select: none; }
.graph-edge { color: var(--viz-edge); }
.graph-edge .edge-arrow { fill: currentColor; stroke: none; }
.graph-edge.updated { color: var(--viz-changed); }
.graph-edge.current { color: var(--viz-reference); }
.graph-edge text { font-size: 15px; font-weight: 800; fill: #263747; paint-order: stroke; stroke: #fbf7ea; stroke-width: 6px; stroke-linejoin: round; }
.graph-node .node-body { fill: #fffdf5; stroke: #334454; stroke-width: 3; }
.graph-node.updated .node-body { fill: var(--viz-changed-fill); stroke: var(--viz-changed); stroke-width: 4; }
.graph-node .reference-ring { fill: none; stroke-width: 3; }
.graph-node .primary { stroke: var(--viz-reference); }
.graph-node .secondary { stroke: var(--viz-reference-other); }
.graph-node text { fill: #263747; font: 700 18px Consolas, monospace; }
.graph-state-list { max-height: 320px; overflow: auto; }
.graph-state-entry { border-top: 1px solid #33445430; padding: 8px 0; overflow-wrap: anywhere; }
.graph-state-entry.updated { background: var(--viz-changed-fill); }
dl { display: grid; grid-template-columns: minmax(80px, 1fr) minmax(0, 2fr); gap: 5px 12px; margin: 8px 0 0; }
dt { font-weight: 700; }
dd { margin: 0; font-family: Consolas, monospace; }
</style>
