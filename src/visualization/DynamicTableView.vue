<script setup lang="ts">
import { computed } from "vue";
import type { DynamicTableState } from "../runtime/dynamicTable";
import type { VizScalar } from "../runtime/visualization";
import { formatScalar } from "../runtime/visualization";
import "./visualization.css";

const props = withDefaults(defineProps<{ state: DynamicTableState; previous?: DynamicTableState; name?: string }>(), { name: "" });
const comparable = computed(() => props.previous?.objectId === props.state.objectId ? props.previous : undefined);
const oldColumns = computed(() => new Map(comparable.value?.columns.map((c, i) => [c, i])));
function updated(row: number, column: string, value: VizScalar) {
  if (!comparable.value) return false;
  const i = oldColumns.value.get(column), oldRow = comparable.value.rows[row];
  return i === undefined || !oldRow || JSON.stringify(oldRow[i]) !== JSON.stringify(value);
}
const display = (v: VizScalar) => typeof v === "string" ? v : formatScalar(v);
</script>

<template>
  <section class="dynamic-table-view viz-state-view" :aria-label="`动态表格${name ? ` · ${name}` : ''}`">
    <header><strong>动态表格<template v-if="name"> · {{ name }}</template></strong><small>{{ state.rows.length }} 行 · {{ state.columns.length }} 列</small></header>
    <div v-if="state.rows.length && state.columns.length" class="dynamic-table-scroll" tabindex="0" aria-label="滚动查看表格">
      <table>
        <thead><tr><th scope="col" class="row-index">行</th><th v-for="c in state.columns" :key="c" scope="col">{{ c }}</th></tr></thead>
        <tbody><tr v-for="(row, r) in state.rows" :key="r"><th scope="row" class="row-index">{{ r }}</th><td v-for="(v, c) in row" :key="state.columns[c]" :class="{ changed: updated(r, state.columns[c], v) }" :title="updated(r, state.columns[c], v) ? '值已变化' : undefined">{{ display(v) }}</td></tr></tbody>
      </table>
    </div>
    <p v-else>{{ state.rows.length ? '当前记录没有字段。' : '空表格 · []' }}</p>
  </section>
</template>

<style scoped>
.dynamic-table-view { min-width: 0; max-width: 100%; margin: 16px 0; border-top: 3px solid #334454; padding-top: 12px; color: #263747; }
header { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: baseline; gap: 8px; margin-bottom: 10px; }
header strong { font-family: Georgia, serif; overflow-wrap: anywhere; }
header small { color: #4b5964; font-size: .8rem; }
.dynamic-table-scroll { max-width: 100%; max-height: 320px; overflow: auto; border: 2px solid #334454; border-radius: 7px; background: #fffdf5; }
.dynamic-table-scroll:focus-visible { outline: 3px solid #235877; outline-offset: 3px; }
table { width: 100%; border-collapse: collapse; font-size: .9rem; }
th, td { padding: 8px 12px; border-bottom: 1px solid #33445430; text-align: left; min-width: 65px; max-width: 320px; overflow-wrap: anywhere; white-space: pre-wrap; }
thead th { position: sticky; top: 0; background: #e2edf1; z-index: 1; }
.row-index { min-width: 36px; color: #4b5964; background: #f8f1df; font-size: .8rem; }
td.changed { background: var(--viz-changed-fill); box-shadow: inset 0 -3px var(--viz-changed); }
</style>
