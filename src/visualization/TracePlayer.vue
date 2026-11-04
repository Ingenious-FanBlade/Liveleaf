<script setup lang="ts">
import { computed, ref, watch, onBeforeUnmount, nextTick } from "vue";
import type { VisualizationTrace, VisualizationFrame } from "../runtime/visualization";
import ArrayView from "./ArrayView.vue";
import TreeView from "./TreeView.vue";
import GraphView from "./GraphView.vue";
import DynamicTableView from "./DynamicTableView.vue";
import { highlightCodeLines } from "../code/syntaxHighlight";
const props = defineProps<{ trace: VisualizationTrace; source: string; documentLine?: number; runStatus: string; stale?: boolean }>();
const index = ref(0);
const playing = ref(false);
const speed = ref(1);
const sourceElement = ref<HTMLElement>();
let timer: ReturnType<typeof setInterval> | undefined;
const frame = computed(() => props.trace.frames[index.value]);
const previous = computed<VisualizationFrame | undefined>(() => {
  const f = frame.value, p = props.trace.frames[index.value - 1];
  if (!f || !p || f.status !== "ready" || p.status !== "ready" || f.position.scopeFrameId !== p.position.scopeFrameId || f.type !== p.type) return;
  if (f.type === "array" && p.type === "array" && f.state?.objectId === p.state?.objectId) return p;
  if (f.type === "tree" && p.type === "tree" && f.state?.rootId === p.state?.rootId) return p;
  if (f.type === "graph" && p.type === "graph" && f.state?.objectId === p.state?.objectId) return p;
  if (f.type === "dynamic-table" && p.type === "dynamic-table" && f.state?.objectId === p.state?.objectId) return p;
});
const previousTables = computed(() => {
  const p = props.trace.frames[index.value - 1];
  return p?.position.scopeFrameId === frame.value?.position.scopeFrameId ? p?.tables : undefined;
});
const scopeChanged = computed(() => index.value > 0 && frame.value?.position.scopeFrameId !== props.trace.frames[index.value - 1].position.scopeFrameId);
const documentLine = computed(() => frame.value?.position.documentLine ?? (props.documentLine && frame.value ? props.documentLine + frame.value.position.cellLine : undefined));
const caption = computed(() => {
  const f = frame.value;
  if (!f) return "没有可回放的状态";
  const line = f.position.cellLine;
  return { call: `进入 ${f.position.functionName}`, line: `即将执行第 ${line} 行`, return: "离开当前调用（可能正在异常展开）", exception: `第 ${line} 行发生异常` }[f.event];
});
const lines = computed(() => highlightCodeLines(props.source.replace(/\r\n?/g, "\n"), "python"));
function stop() { playing.value = false; if (timer) clearInterval(timer); timer = undefined; }
function seek(value: number) { stop(); index.value = Math.max(0, Math.min(value, props.trace.frames.length - 1)); }
function play() {
  if (playing.value) { stop(); return; }
  if (index.value >= props.trace.frames.length - 1) index.value = 0;
  playing.value = true;
  timer = setInterval(() => {
    if (index.value >= props.trace.frames.length - 1) stop();
    else index.value++;
  }, 650 / speed.value);
}
watch(() => props.trace.runId, () => { stop(); const first = props.trace.frames.findIndex(f => f.status === "ready"); index.value = Math.max(0, first); }, { immediate: true });
watch(speed, () => { if (playing.value) { stop(); play(); } });
watch(index, () => nextTick(() => {
  const active = sourceElement.value?.querySelector<HTMLElement>(".active-line");
  if (active && sourceElement.value) {
    const pre = sourceElement.value;
    pre.scrollTop += active.getBoundingClientRect().top - pre.getBoundingClientRect().top - 48;
  }
}));
onBeforeUnmount(stop);
</script>

<template>
  <section class="trace-player" aria-label="Python 状态回放" @keydown.stop>
    <header class="trace-heading"><strong>{{ { array: '数组', tree: '树', graph: '有向图', 'dynamic-table': '动态表格' }[trace.config.type] }} · {{ trace.config.watch }}</strong><span>执行后回放</span></header>
    <p v-if="stale" class="trace-notice">结果已过期，以下为运行时的源码与状态。请重新运行以更新。</p>
    <p v-if="runStatus !== 'success'" class="trace-notice">程序{{ runStatus === 'timeout' ? '超时' : '未正常结束' }}，可回放已记录的部分。</p>
    <p v-if="trace.capture.status !== 'complete'" class="trace-notice">采集{{ trace.capture.status === 'truncated' ? '已截断' : '出错' }} · {{ trace.capture.reason }}</p>
    <template v-if="frame">
      <div class="trace-controls">
        <button type="button" :disabled="index === 0" aria-label="前一事件" @click="seek(index - 1)">← 上一步</button>
        <button type="button" :disabled="trace.frames.length < 2" :aria-pressed="playing" @click="play">{{ playing ? '暂停' : '播放' }}</button>
        <button type="button" :disabled="index >= trace.frames.length - 1" aria-label="后一事件" @click="seek(index + 1)">下一步 →</button>
        <label>速度 <select v-model.number="speed" aria-label="回放速度"><option v-for="s in [.5, 1, 2, 4]" :key="s" :value="s">{{ s }}×</option></select></label>
      </div>
      <div class="trace-progress"><input type="range" min="0" :max="trace.frames.length - 1" :value="index" aria-label="定位采集事件" @input="seek(Number(($event.target as HTMLInputElement).value))" /><small>{{ index + 1 }} / {{ trace.frames.length }}</small></div>
      <p class="trace-position">{{ caption }}<br /><small :title="`事件 ${frame.position.frameId} / 作用域 ${frame.position.scopeFrameId}`">{{ frame.position.functionName }}<template v-if="documentLine"> · 文档第 {{ documentLine }} 行</template></small></p>
      <p v-if="scopeChanged" class="scope-change">已切换作用域调用，变化比较已重置。</p>
      <ArrayView v-if="frame.type === 'array' && frame.state" :state="frame.state" :pointers="frame.pointers" :previous="previous?.type === 'array' ? previous.state ?? undefined : undefined" />
      <TreeView v-else-if="frame.type === 'tree' && frame.state" :state="frame.state" :pointers="frame.pointers" :previous="previous?.type === 'tree' ? previous.state ?? undefined : undefined" />
      <GraphView v-else-if="frame.type === 'graph' && frame.state" :state="frame.state" :pointers="frame.pointers" :previous="previous?.type === 'graph' ? previous.state ?? undefined : undefined" />
      <DynamicTableView v-else-if="frame.type === 'dynamic-table' && frame.state" :state="frame.state" :name="trace.config.watch" :previous="previous?.type === 'dynamic-table' ? previous.state ?? undefined : undefined" />
      <p v-else class="trace-notice">{{ frame.status === 'unbound' ? '观察变量尚未绑定。' : '当前主体不可用。' }} {{ frame.diagnosticCodes.join(', ') }}</p>
      <template v-for="table in frame.tables" :key="table.name">
        <DynamicTableView v-if="table.state" :name="table.name" :state="table.state" :previous="previousTables?.find(t => t.name === table.name)?.state ?? undefined" />
        <p v-else class="trace-notice">动态表格 · {{ table.name }}：{{ table.status === 'unbound' ? '观察变量尚未绑定。' : '当前数据不可用，请检查绑定的数组。' }}</p>
      </template>
      <details open class="trace-source"><summary>运行时源码 · {{ caption }}</summary><pre ref="sourceElement" class="sl-code-theme"><code><span v-for="(line, i) in lines" :key="i" class="source-line" :class="{ 'active-line': i + 1 === frame.position.cellLine }"><span class="line-number">{{ i + 1 }}</span><span v-html="line || ' '" />
</span></code></pre></details>
    </template>
    <p v-else>没有可回放的状态。</p>
    <details v-if="trace.diagnostics.length" class="trace-diagnostics" :open="!frame"><summary>采集诊断（{{ trace.diagnostics.length }}）</summary><p v-for="(d, i) in trace.diagnostics" :key="i"><strong>{{ d.code }}</strong> · {{ d.message }}</p></details>
  </section>
</template>

<style scoped>
.trace-player { min-width: 0; max-width: 100%; margin-top: 14px; color: #263747; font-family: system-ui, sans-serif; font-weight: 600; }
.trace-heading { display: flex; gap: 12px; justify-content: space-between; border-bottom: 3px solid #334454; padding-bottom: 10px; }
.trace-heading strong { font-family: Georgia, serif; font-size: 1.15rem; }
.trace-heading span { font-size: .75rem; align-self: center; }
.trace-controls { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin: 14px 0 8px; }
.trace-controls button, select { min-height: 36px; padding: 4px 10px; border: 2px solid #334454; border-radius: 7px; background: #fffdf5; color: #263747; font: inherit; cursor: pointer; }
.trace-controls button:nth-child(2) { background: #f3b642; box-shadow: 3px 3px 0 #334454; }
.trace-controls button:disabled { opacity: .45; cursor: default; }
.trace-controls label { font-size: .8rem; }
.trace-progress { display: flex; gap: 10px; align-items: center; }
.trace-progress input { width: 100%; min-width: 0; accent-color: #235877; }
.trace-progress small { white-space: nowrap; }
.trace-position { line-height: 1.5; margin: 12px 0; }
.trace-position small { color: #4b5964; font-size: .85rem; overflow-wrap: anywhere; }
.trace-notice, .scope-change { margin: 10px 0; padding: 8px 10px; border-left: 4px solid #a1362d; background: #fff0e5; font-size: .85rem; }
.scope-change { border-color: #235877; background: #e2f2fa; }
.trace-source { margin-top: 14px; font-size: .8rem; }
summary { cursor: pointer; }
.trace-source pre { position: relative; padding: 8px 0 !important; max-height: 210px; overflow: auto; margin: 8px 0 !important; border: 2px solid #334454; border-radius: 6px; background: #334a5e !important; color: #fffdf5 !important; line-height: 1.6 !important; }
.source-line { display: block; min-width: max-content; padding-right: 12px; }
.line-number { display: inline-block; width: 3.2em; padding-right: 10px; text-align: right; opacity: .75; user-select: none; }
.trace-diagnostics { margin-top: 12px; font-size: .78rem; overflow-wrap: anywhere; }
</style>
