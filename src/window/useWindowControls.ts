import { onBeforeUnmount, onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";

export type WindowResizeDirection = Parameters<ReturnType<typeof getCurrentWindow>["startResizeDragging"]>[0];

export function useWindowControls() {
  const desktop = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  const appWindow = desktop ? getCurrentWindow() : undefined;
  const maximized = ref(false);
  const fullscreen = ref(false);
  const focused = ref(true);
  const busy = ref(false);
  const error = ref("");
  const unlisteners: UnlistenFn[] = [];
  let disposed = false;
  let revision = 0;

  async function refresh() {
    if (!appWindow) return;
    const current = ++revision;
    const state = await Promise.all([appWindow.isMaximized(), appWindow.isFullscreen(), appWindow.isFocused()]);
    if (!disposed && current === revision) [maximized.value, fullscreen.value, focused.value] = state;
  }

  async function observe(subscription: Promise<UnlistenFn>) {
    try {
      const unlisten = await subscription;
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    } catch (cause) {
      console.error("Unable to observe window state", cause);
      if (!disposed) error.value = "无法同步窗口状态，请重新打开应用。";
    }
  }

  async function sync() {
    try { await refresh(); }
    catch (cause) {
      console.error("Unable to read window state", cause);
      if (!disposed) error.value = "无法读取窗口状态，请重新打开应用。";
    }
  }

  async function perform(label: string, action: () => Promise<void>) {
    if (!appWindow || busy.value) return;
    busy.value = true;
    error.value = "";
    try { await action(); }
    catch (cause) {
      console.error(`Unable to ${label}`, cause);
      if (!disposed) error.value = `无法${label}窗口，请重试。`;
    } finally { busy.value = false; }
  }

  const minimize = () => perform("最小化", () => appWindow!.minimize());
  const toggleMaximize = () => {
    if (fullscreen.value) return;
    return perform(maximized.value ? "还原" : "最大化", async () => {
      await appWindow!.toggleMaximize();
      await refresh();
    });
  };
  const close = () => perform("关闭", () => appWindow!.close());
  const resize = (event: MouseEvent, direction: WindowResizeDirection) => {
    if (event.button !== 0 || maximized.value || fullscreen.value) return;
    return perform("调整", () => appWindow!.startResizeDragging(direction));
  };

  onMounted(() => {
    if (!appWindow) return;
    void sync();
    void observe(appWindow.onResized(() => { void sync(); }));
    void observe(appWindow.onFocusChanged(({ payload }) => { if (!disposed) focused.value = payload; }));
  });
  onBeforeUnmount(() => {
    disposed = true;
    for (const unlisten of unlisteners) unlisten();
  });

  return { desktop, maximized, fullscreen, focused, busy, error, minimize, toggleMaximize, close, resize };
}
