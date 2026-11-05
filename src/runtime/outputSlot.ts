/**
 * Run output is teleported into slots inside a slide's `v-html` content. Whenever
 * that HTML changes, `v-html` rebuilds the DOM and discards the slots, but a Teleport
 * only looks up its target again when `to` changes. Tying the selector to the HTML
 * makes Vue move the output into the new slot, keeping replay state intact.
 */
export function slideRenderKey(html: string) {
  let hash = 2166136261;

  for (let index = 0; index < html.length; index += 1) {
    hash = Math.imul(hash ^ html.charCodeAt(index), 16777619);
  }

  return (hash >>> 0).toString(36);
}

export function runtimeOutputTarget(slideHtml: string, cellId: string) {
  return `[data-render-key="${slideRenderKey(slideHtml)}"] [data-code-cell-output="${encodeURIComponent(cellId)}"]`;
}
