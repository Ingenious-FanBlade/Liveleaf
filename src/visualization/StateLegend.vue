<script setup lang="ts">
withDefaults(defineProps<{ references?: number }>(), { references: 0 });
</script>

<template>
  <div class="state-legend" aria-label="可视化颜色图例">
    <span v-if="references"><i class="reference" aria-hidden="true" />当前引用</span>
    <span v-if="references > 1"><i class="reference-other" aria-hidden="true" />其他引用</span>
    <span><i class="changed" aria-hidden="true" />状态变化</span>
  </div>
</template>

<style scoped>
.state-legend { display: flex; flex-wrap: wrap; gap: 10px 18px; align-items: center; margin: 12px 0; color: #334454; font-size: .88rem; line-height: 1.5; }
.state-legend span { display: inline-flex; align-items: center; gap: 7px; }
i { display: inline-block; width: 14px; height: 14px; border: 2px solid; border-radius: 4px; flex: none; }
.reference { border-color: var(--viz-reference); background: #dceefa; }
.reference-other { border-color: var(--viz-reference-other); background: #f9dfd5; }
.changed { border-color: var(--viz-changed); background: var(--viz-changed-fill); }
</style>
