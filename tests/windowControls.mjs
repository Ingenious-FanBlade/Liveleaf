import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { webcrypto } from "node:crypto";
import { createRenderer, createSSRApp, h } from "vue";
import { renderToString } from "vue/server-renderer";
import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { createServer } from "vite";

const server = await createServer({ server: { middlewareMode: true }, appType: "custom" });
const previousWindow = globalThis.window;
try {
  const config = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
  const capability = JSON.parse(await readFile(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"));
  assert.equal(config.app.windows[0].decorations, false);
  assert.equal(config.app.windows[0].resizable, true);
  for (const permission of ["allow-minimize", "allow-toggle-maximize", "allow-close", "allow-start-dragging", "allow-start-resize-dragging"]) assert.ok(capability.permissions.includes(`core:window:${permission}`), `Window control must have native permission: ${permission}`);

  const { useWindowControls } = await server.ssrLoadModule("/src/window/useWindowControls.ts");
  const WindowFrame = (await server.ssrLoadModule("/src/window/WindowFrame.vue")).default;
  const html = await renderToString(createSSRApp({ render: () => h(WindowFrame, { title: "Lesson <script>" }, () => h("main", "Course")) }));
  assert.match(html, /data-tauri-drag-region/);
  assert.equal((html.match(/ disabled(?:[\s>])/g) ?? []).length, 3, "Browser preview must not invoke desktop window APIs");
  assert.match(html, /Lesson &lt;script&gt;/);
  assert.match(html, /<main[^>]*>Course<\/main>/);

  globalThis.window = { crypto: webcrypto };
  mockWindows("main");
  const native = { maximized: false, fullscreen: false, focused: true };
  const calls = [];
  let failMinimize = false;
  mockIPC((cmd, payload) => {
    calls.push([cmd, payload]);
    switch (cmd) {
      case "plugin:window|is_maximized": return native.maximized;
      case "plugin:window|is_fullscreen": return native.fullscreen;
      case "plugin:window|is_focused": return native.focused;
      case "plugin:window|toggle_maximize": native.maximized = !native.maximized; return;
      case "plugin:window|minimize": if (failMinimize) throw new Error("Native minimize failed"); return;
      case "plugin:window|close": return;
      case "plugin:window|start_resize_dragging": return;
      default: throw new Error(`Unexpected native command: ${cmd}`);
    }
  }, { shouldMockEvents: true });

  // Mount the controller with Vue's real lifecycle, without a browser DOM.
  const node = () => ({ children: [], parent: null });
  const renderer = createRenderer({
    createElement: node, createText: node, createComment: node,
    insert(child, parent) { child.parent = parent; parent.children.push(child); },
    remove(child) { const p = child.parent; if (p) p.children.splice(p.children.indexOf(child), 1); },
    parentNode: child => child.parent, nextSibling: () => null,
    patchProp() {}, setElementText() {}, setText() {},
  });
  let controls;
  const app = renderer.createApp({ setup() { controls = useWindowControls(); return () => h("div"); } });
  const settle = () => new Promise(resolve => setImmediate(resolve));
  app.mount(node());
  await settle();
  assert.equal(controls.desktop, true);
  assert.equal(controls.maximized.value, false);
  await controls.toggleMaximize();
  assert.equal(controls.maximized.value, true);
  await controls.toggleMaximize();
  assert.equal(controls.maximized.value, false);

  native.maximized = true;
  await emit("tauri://resize", { width: 1280, height: 720 });
  await settle();
  assert.equal(controls.maximized.value, true, "Native double-click/Snap/restore events must update the maximize button");
  const before = calls.length;
  await controls.resize({ button: 0 }, "SouthEast");
  assert.equal(calls.length, before, "Maximized windows cannot start edge resizing");
  native.maximized = false;
  await emit("tauri://resize", { width: 800, height: 600 });
  await settle();
  await controls.resize({ button: 2 }, "SouthEast");
  assert.equal(calls.filter(([c]) => c === "plugin:window|start_resize_dragging").length, 0);
  await controls.resize({ button: 0 }, "SouthEast");
  assert.equal(calls.at(-1)[1].value, "SouthEast");

  await emit("tauri://blur");
  assert.equal(controls.focused.value, false);
  await controls.minimize();
  assert.equal(calls.at(-1)[0], "plugin:window|minimize");
  await controls.close();
  assert.equal(calls.at(-1)[0], "plugin:window|close");
  failMinimize = true;
  const originalError = console.error;
  try { console.error = () => {}; await controls.minimize(); }
  finally { console.error = originalError; }
  assert.match(controls.error.value, /无法最小化窗口/);
  assert.equal(controls.busy.value, false, "A failed native operation must leave controls usable");

  native.fullscreen = true;
  await emit("tauri://resize", { width: 1280, height: 720 });
  await settle();
  const inFullscreen = calls.length;
  await controls.toggleMaximize();
  await controls.resize({ button: 0 }, "West");
  assert.equal(calls.length, inFullscreen, "Fullscreen must not start resize or maximize operations");
  const callbacks = window.__TAURI_INTERNALS__.callbacks;
  assert.equal(callbacks.size, 3, "The controller subscribes to resize, focus, and blur");
  app.unmount();
  await settle();
  assert.equal(callbacks.size, 0, "All native event callbacks must be released on unmount");
  console.log("Passed frameless window controls, native permissions, state synchronization, lifecycle, and browser fallback checks.");
} finally {
  clearMocks();
  if (previousWindow === undefined) delete globalThis.window;
  else globalThis.window = previousWindow;
  await server.close();
}
