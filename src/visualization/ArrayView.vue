<script setup lang="ts">
import { computed } from "vue";
import { formatScalar } from "../runtime/visualization";
import type { ArrayState, ArrayPointer } from "../runtime/visualization";
import StateLegend from "./StateLegend.vue";
import "./visualization.css";
const props = defineProps<{ state: ArrayState; pointers: ArrayPointer[]; previous?: ArrayState }>();
const changed = computed(() => props.state.items.map((v, i) => props.previous && JSON.stringify(v) !== JSON.stringify(props.previous.items[i])));
const namesAt = (index: number) => props.pointers.filter(p => p.status === "inBounds" && p.index === index).map(p => `${p.name}=${formatScalar(p.raw)}`).join(", ");
const primaryAt = (index: number) => props.pointers[0]?.status === "inBounds" && props.pointers[0].index === index;
const secondaryAt = (index: number) => props.pointers.slice(1).some(p => p.status === "inBounds" && p.index === index);
</script>

<template>
  <div class="array-view viz-state-view" role="group" aria-label="数组状态">
    <p v-if="!state.items.length">空数组 []</p>
    <div v-else class="array-strip">
      <div v-for="(item, index) in state.items" :key="index" class="array-item" :class="{ changed: changed[index], primary: primaryAt(index), secondary: secondaryAt(index) }">
        <strong :title="`${formatScalar(item)}${namesAt(index) ? ` · ${namesAt(index)}` : ''}${changed[index] ? ' · 已变化' : ''}`">{{ formatScalar(item) }}</strong>
        <small>[{{ index }}]</small>
      </div>
    </div>
    <StateLegend :references="pointers.length" />
    <details v-if="pointers.length" class="viz-details"><summary>观察详情</summary><p v-for="p in pointers" :key="p.name">{{ p.name }}：{{ p.status === 'inBounds' ? `${formatScalar(p.raw)} → [${p.index}]` : p.status === 'outOfBounds' ? `${formatScalar(p.raw)} · 越界` : p.status === 'unbound' ? '未绑定' : '无效类型' }}</p></details>
  </div>
</template>

<style scoped>
.array-view { max-width: 100%; min-width: 0; padding: 12px 2px; }
.array-strip { display: flex; align-items: flex-end; gap: 12px; max-width: 100%; overflow-x: auto; padding: 6px 4px 10px; }
.array-item { display: grid; gap: 6px; text-align: center; min-width: 64px; max-width: 150px; flex: 0 0 auto; }
.array-item strong { display: block; padding: 14px 10px; border: 3px solid var(--ink, #334454); border-radius: 8px; background: var(--surface, #fffdf5); box-shadow: 3px 3px 0 #33445425; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.changed strong { background: var(--viz-changed-fill); }
.primary strong { border-color: var(--viz-reference); }
.secondary strong { outline: 3px solid var(--viz-reference-other); outline-offset: 2px; }
small { font-size: .85rem; }
</style>
