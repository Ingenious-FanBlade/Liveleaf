<script setup lang="ts">
import { computed, ref } from "vue";
import type { TreeState, TreePointer, TreeNode } from "../runtime/visualization";
import { formatScalar } from "../runtime/visualization";
import StateLegend from "./StateLegend.vue";
import "./visualization.css";
const props = defineProps<{ state: TreeState; pointers: TreePointer[]; previous?: TreeState }>();
const zoom = ref(1);
const viewport = ref<HTMLElement>();
let drag: { x: number; y: number; left: number; top: number } | undefined;
const layout = computed(() => {
  const map = new Map(props.state.nodes.map(n => [n.id, n]));
  const positions = new Map<string, { x: number; y: number; node: TreeNode }>();
  let cursor = 0;
  function place(id: string | null, depth: number): number {
    if (id === null) return (cursor++ * 112) + 64;
    const node = map.get(id)!;
    const slots = node.children.map(e => place(e.nodeId, depth + 1));
    const x = slots.length ? (slots[0] + slots[slots.length - 1]) / 2 : (cursor++ * 112) + 64;
    positions.set(id, { x, y: depth * 116 + 64, node });
    return x;
  }
  if (props.state.rootId) place(props.state.rootId, 0);
  const nodes = [...positions.values()];
  const edges = nodes.flatMap(p => p.node.children.filter(e => e.nodeId).map(e => ({ from: p, to: positions.get(e.nodeId!)!, label: e.field + (e.index === null ? "" : `[${e.index}]`) })));
  return { nodes, edges, width: Math.max(320, cursor * 112 + 16), height: Math.max(140, ...nodes.map(p => p.y + 64)) };
});
const previous = computed(() => new Map(props.previous?.nodes.map(n => [n.id, n])));
const changed = (node: TreeNode) => props.previous && JSON.stringify(previous.value.get(node.id)) !== JSON.stringify(node);
const namesAt = (id: string) => props.pointers.filter(p => p.status === "matched" && p.nodeId === id).map(p => p.name).join(", ");
const primaryAt = (id: string) => props.pointers[0]?.status === "matched" && props.pointers[0].nodeId === id;
const secondaryAt = (id: string) => props.pointers.slice(1).some(p => p.status === "matched" && p.nodeId === id);
const short = (node: TreeNode) => { const s = Array.from(formatScalar(node.value)); return s.length > 10 ? s.slice(0, 9).join("") + "…" : s.join(""); };
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
</script>

<template>
  <div class="tree-view viz-state-view">
    <div class="zoom-controls" aria-label="树视图缩放">
      <button type="button" :disabled="zoom <= .3" aria-label="缩小树" @click="zoom = Math.max(.3, zoom - .2)">−</button>
      <button type="button" aria-label="重置缩放" @click="zoom = 1">{{ Math.round(zoom * 100) }}%</button>
      <button type="button" :disabled="zoom >= 2" aria-label="放大树" @click="zoom = Math.min(2, zoom + .2)">+</button>
      <small>拖动画布或滚动查看</small>
    </div>
    <p v-if="state.rootId === null">空树 · None</p>
    <div v-else ref="viewport" class="tree-viewport" tabindex="0" aria-label="树结构，支持滚动和平移" @pointerdown="start" @pointermove="move" @pointerup="drag = undefined" @pointercancel="drag = undefined" @lostpointercapture="drag = undefined">
      <svg :width="layout.width * zoom" :height="layout.height * zoom" :viewBox="`0 0 ${layout.width} ${layout.height}`" role="img" aria-label="有序树及节点引用">
        <g v-for="(edge, i) in layout.edges" :key="i" class="tree-edge">
          <title>{{ edge.label }}</title>
          <path :d="`M ${edge.from.x} ${edge.from.y + 25} L ${edge.to.x} ${edge.to.y - 25}`" />
        </g>
        <g v-for="p in layout.nodes" :key="p.node.id" :transform="`translate(${p.x}, ${p.y})`" class="tree-node" :class="{ changed: changed(p.node) }">
          <title>{{ formatScalar(p.node.value) }} · {{ p.node.id }}{{ namesAt(p.node.id) ? ` · ${namesAt(p.node.id)}` : '' }}{{ changed(p.node) ? ' · 已变化' : '' }}</title>
          <rect class="node-body" x="-47" y="-25" width="94" height="50" rx="14" />
          <rect v-if="primaryAt(p.node.id)" class="reference-ring primary" x="-51" y="-29" width="102" height="58" rx="18" />
          <rect v-if="secondaryAt(p.node.id)" class="reference-ring secondary" :x="primaryAt(p.node.id) ? -55 : -51" :y="primaryAt(p.node.id) ? -33 : -29" :width="primaryAt(p.node.id) ? 110 : 102" :height="primaryAt(p.node.id) ? 66 : 58" rx="20" />
          <text text-anchor="middle" dominant-baseline="middle">{{ short(p.node) }}</text>
        </g>
      </svg>
    </div>
    <StateLegend :references="pointers.length" />
    <details v-if="pointers.length" class="viz-details"><summary>观察详情</summary><p v-for="p in pointers" :key="p.name">{{ p.name }}：{{ p.nodeId ? formatScalar(state.nodes.find(n => n.id === p.nodeId)!.value) : { null: '空引用', outside: '树外引用', unbound: '未绑定', invalidType: '无效类型', matched: '' }[p.status] }}</p></details>
  </div>
</template>

<style scoped>
.tree-view { min-width: 0; }
.tree-viewport { overflow: auto; max-width: 100%; max-height: 420px; cursor: grab; touch-action: none; background-image: radial-gradient(#33445418 1px, transparent 1px); background-size: 20px 20px; }
.tree-viewport:active { cursor: grabbing; }
svg { display: block; max-width: none; user-select: none; }
.tree-edge path { stroke: #334454; stroke-width: 3; stroke-linecap: round; fill: none; }
.tree-node .node-body { stroke: #334454; stroke-width: 3; fill: #fffdf5; }
.tree-node text { fill: #263747; font-size: 16px; font-family: Consolas, monospace; }
.tree-node.changed .node-body { fill: var(--viz-changed-fill); stroke: var(--viz-changed); }
.tree-node .reference-ring { fill: none; stroke-width: 3; }
.tree-node .primary { stroke: var(--viz-reference); }
.tree-node .secondary { stroke: var(--viz-reference-other); }
.zoom-controls { display: flex; align-items: center; flex-wrap: wrap; gap: 6px; padding: 8px 0; }
button { min-height: 32px; min-width: 34px; border: 2px solid #334454; border-radius: 6px; background: #fffdf5; color: #263747; cursor: pointer; }
small { font-size: .88rem; }
button:disabled { opacity: .5; cursor: default; }
</style>
