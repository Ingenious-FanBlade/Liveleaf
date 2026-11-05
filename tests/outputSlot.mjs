import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createServer } from "vite";

const server = await createServer({ server: { middlewareMode: true }, appType: "custom" });
try {
  const { runtimeOutputTarget, slideRenderKey } = await server.ssrLoadModule("/src/runtime/outputSlot.ts");
  const { parseCourse } = await server.ssrLoadModule("/src/markdown/parseCourse.ts");
  const htmlFor = code => parseCourse(`\`\`\`python run id="a b"\n${code}\n\`\`\`\n`).slides[0].html;

  const original = htmlFor("print(1)");
  const edited = htmlFor("print(2)");

  assert.equal(runtimeOutputTarget(original, "a b"), runtimeOutputTarget(htmlFor("print(1)"), "a b"), "Unchanged HTML keeps its slot, so the output is left in place");
  assert.notEqual(runtimeOutputTarget(original, "a b"), runtimeOutputTarget(edited, "a b"), "Rebuilt HTML must change the Teleport target, or the output stays in the discarded DOM");
  assert.notEqual(slideRenderKey("ab"), slideRenderKey("ba"), "The key depends on character order");

  const slot = original.match(/data-code-cell-output="([^"]*)"/)?.[1];
  assert.ok(runtimeOutputTarget(original, "a b").endsWith(`[data-code-cell-output="${slot}"]`), "The selector must match the slot parseCourse renders");

  const app = await readFile(new URL("../src/App.vue", import.meta.url), "utf8");
  for (const name of ["slide", "activeSlide"]) {
    assert.ok(app.includes(`:data-render-key="slideRenderKey(${name}.html)"`), `The ${name} v-html container must expose its render key`);
    assert.ok(app.includes(`:to="runtimeOutputTarget(${name}.html, cell.id)"`), `The ${name} Teleport must target the current render`);
  }

  console.log("Passed run output slot checks: output follows rebuilt slide HTML in preview and presentation.");
} finally {
  await server.close();
}
