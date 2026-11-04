<script setup lang="ts">
import { ref } from "vue";
import { useWindowControls } from "./useWindowControls";
import type { WindowResizeDirection } from "./useWindowControls";
// The 32-grid app icon variant stays crisp at title-bar size.
// Icon sources live under src/ (not src-tauri/, which Vite does not watch) so edits hot-reload.
import brandIcon from "../assets/brand/liveleaf-small.svg";

defineProps<{ title: string }>();
const element = ref<HTMLElement>();
defineExpose({ element });
const { desktop, maximized, fullscreen, focused, busy, error, minimize, toggleMaximize, close, resize } = useWindowControls();
const edges: WindowResizeDirection[] = ["North", "South", "East", "West", "NorthEast", "NorthWest", "SouthEast", "SouthWest"];
</script>

<template>
  <div ref="element" class="desktop-window" :class="{ 'window-maximized': maximized, 'window-fullscreen': fullscreen, 'window-inactive': !focused }">
    <header class="window-titlebar" aria-label="窗口标题栏">
      <div class="window-drag-region" data-tauri-drag-region title="拖动窗口，双击最大化或还原">
        <img class="window-brand-mark" :src="brandIcon" alt="" aria-hidden="true" draggable="false" />
        <span class="window-brand">Liveleaf</span>
        <span class="window-document-title">{{ title }}</span>
      </div>
      <div class="window-controls" aria-label="窗口操作">
        <button type="button" class="window-control" aria-label="最小化窗口" title="最小化" :disabled="!desktop || busy" @click="minimize"><svg viewBox="0 0 20 20" aria-hidden="true"><path d="M5 13h10" /></svg></button>
        <button type="button" class="window-control" :aria-label="maximized ? '还原窗口' : '最大化窗口'" :title="maximized ? '还原' : '最大化'" :aria-pressed="maximized" :disabled="!desktop || busy || fullscreen" @click="toggleMaximize">
          <svg v-if="maximized" viewBox="0 0 20 20" aria-hidden="true"><path d="M7 6V4h9v9h-2M4 7h9v9H4z" /></svg>
          <svg v-else viewBox="0 0 20 20" aria-hidden="true"><path d="M5 5h10v10H5z" /></svg>
        </button>
        <button type="button" class="window-control window-control-close" aria-label="关闭窗口" title="关闭" :disabled="!desktop || busy" @click="close"><svg viewBox="0 0 20 20" aria-hidden="true"><path d="m5 5 10 10M15 5 5 15" /></svg></button>
      </div>
    </header>
    <p v-if="error" class="window-error" role="alert">{{ error }}</p>
    <div class="window-content"><slot /></div>
    <template v-if="desktop && !maximized && !fullscreen">
      <div v-for="edge in edges" :key="edge" class="window-resize-edge" :class="`resize-${edge.toLowerCase()}`" aria-hidden="true" @mousedown.prevent.stop="resize($event, edge)" />
    </template>
  </div>
</template>

<style scoped>
.desktop-window { position: relative; isolation: isolate; display: flex; flex-direction: column; width: 100%; height: 100dvh; min-height: 0; overflow: hidden; border: 4px solid var(--ink); border-radius: 12px; background: var(--paper); background-clip: padding-box; color: var(--ink-dark); }
.window-titlebar { display: flex; flex: 0 0 48px; min-width: 0; align-items: center; gap: 12px; padding: 5px 10px 7px 12px; border-bottom: 3px solid var(--ink); background: var(--paper-soft); user-select: none; }
.window-drag-region { display: flex; flex: 1; align-self: stretch; min-width: 0; align-items: center; gap: 10px; cursor: default; }
.window-drag-region > * { pointer-events: none; }
.window-brand-mark { display: block; flex: 0 0 28px; width: 28px; height: 28px; }
.window-brand { flex: none; font-family: Georgia, "Times New Roman", serif; font-size: .95rem; font-weight: 700; white-space: nowrap; }
.window-document-title { min-width: 0; margin-left: 5px; padding-left: 14px; overflow: hidden; border-left: 2px solid var(--line-soft); color: #586570; font-size: .8rem; text-overflow: ellipsis; white-space: nowrap; }
.window-controls { display: flex; flex: none; align-items: center; gap: 7px; }
.window-control { display: grid; width: 36px; height: 30px; place-items: center; padding: 0; border: 2px solid var(--ink); border-radius: 6px; background: var(--surface); color: var(--ink-dark); box-shadow: 2px 2px 0 var(--shadow); cursor: pointer; transition: background 140ms, transform 140ms, box-shadow 140ms; }
.window-control svg { width: 18px; height: 18px; fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
.window-control:hover:not(:disabled) { transform: translate(-1px, -1px); background: #dceefa; box-shadow: 3px 3px 0 var(--shadow); }
.window-control:active:not(:disabled) { transform: translate(1px, 1px); box-shadow: none; }
.window-control:focus-visible { outline: 3px solid #235877; outline-offset: 3px; }
.window-control:disabled { opacity: .45; cursor: default; }
.window-control-close { background: var(--red); color: var(--surface); }
.window-control-close:hover:not(:disabled) { background: #c53d34; }
.window-content { flex: 1; min-width: 0; min-height: 0; overflow: hidden; }
.window-error { flex: none; margin: 0; padding: 6px 12px; border-bottom: 2px solid var(--ink); background: #fff0e5; color: #9b2721; font-size: .8rem; }
.window-inactive .window-brand, .window-inactive .window-brand-mark { opacity: .65; }
.window-maximized, .window-fullscreen, .desktop-window:fullscreen { border-radius: 0; }
.window-fullscreen, .desktop-window:fullscreen { border-width: 0; }
.window-resize-edge { position: absolute; z-index: 10; }
.resize-north, .resize-south { height: 5px; left: 12px; right: 12px; cursor: ns-resize; }
.resize-north { top: 0; }
.resize-south { bottom: 0; }
.resize-east, .resize-west { width: 5px; top: 12px; bottom: 12px; cursor: ew-resize; }
.resize-east { right: 0; }
.resize-west { left: 0; }
.resize-northeast, .resize-northwest, .resize-southeast, .resize-southwest { width: 10px; height: 10px; }
.resize-northeast, .resize-northwest { top: 0; }
.resize-southeast, .resize-southwest { bottom: 0; }
.resize-northeast, .resize-southeast { right: 0; }
.resize-northwest, .resize-southwest { left: 0; }
.resize-northeast, .resize-southwest { cursor: nesw-resize; }
.resize-northwest, .resize-southeast { cursor: nwse-resize; }
@media (max-width: 520px) { .window-document-title { display: none; } .window-titlebar { gap: 6px; padding-left: 8px; } .window-brand { font-size: .85rem; } }
@media (prefers-reduced-motion: reduce) { .window-control { transition: none; } }
</style>
