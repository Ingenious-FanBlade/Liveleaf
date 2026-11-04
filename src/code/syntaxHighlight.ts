import { syntaxHighlighting } from "@codemirror/language";
import type { Language } from "@codemirror/language";
import { markdown } from "@codemirror/lang-markdown";
import { pythonLanguage } from "@codemirror/lang-python";
import { javascriptLanguage, jsxLanguage, typescriptLanguage, tsxLanguage } from "@codemirror/lang-javascript";
import { jsonLanguage } from "@codemirror/lang-json";
import { highlightCode as renderHighlight, tagHighlighter, tags } from "@lezer/highlight";

// The editor and every read-only view share both token classes and language parsers.
export const codeHighlighter = tagHighlighter([
  { tag: tags.keyword, class: "sl-syntax-keyword" },
  { tag: [tags.variableName, tags.propertyName], class: "sl-syntax-variable" },
  { tag: [tags.function(tags.variableName), tags.function(tags.propertyName)], class: "sl-syntax-function" },
  { tag: [tags.typeName, tags.className, tags.namespace], class: "sl-syntax-type" },
  { tag: [tags.string, tags.character, tags.regexp], class: "sl-syntax-string" },
  { tag: [tags.number, tags.bool, tags.null, tags.atom], class: "sl-syntax-literal" },
  { tag: tags.comment, class: "sl-syntax-comment" },
  { tag: tags.operator, class: "sl-syntax-operator" },
  { tag: [tags.meta, tags.processingInstruction], class: "sl-syntax-meta" },
  { tag: tags.heading, class: "sl-syntax-heading" },
  { tag: tags.link, class: "sl-syntax-link" },
  { tag: tags.emphasis, class: "sl-syntax-emphasis" },
  { tag: tags.strong, class: "sl-syntax-strong" },
]);

export function codeLanguage(info: string): Language | null {
  const name = info.trim().split(/\s+/, 1)[0].toLowerCase();
  switch (name) {
    case "python": case "python3": case "py": return pythonLanguage;
    case "javascript": case "js": case "mjs": case "cjs": return javascriptLanguage;
    case "typescript": case "ts": return typescriptLanguage;
    case "jsx": return jsxLanguage;
    case "tsx": return tsxLanguage;
    case "json": return jsonLanguage;
    default: return null;
  }
}

export const courseMarkdown = markdown({
  codeLanguages: codeLanguage,
  completeHTMLTags: false,
  pasteURLAsLink: false,
});
export const codeSyntaxHighlighting = syntaxHighlighting(codeHighlighter);

export function escapeCode(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}

/** Render whole syntax trees before splitting lines, so multiline strings stay correct. */
export function highlightCodeLines(source: string, language: string): string[] {
  const parser = codeLanguage(language)?.parser;
  if (!parser) return source.split("\n").map(escapeCode);
  const lines = [""];
  renderHighlight(source, parser.parse(source), codeHighlighter, (text, classes) => {
    const escaped = escapeCode(text);
    lines[lines.length - 1] += classes ? `<span class="${classes}">${escaped}</span>` : escaped;
  }, () => lines.push(""));
  return lines;
}

export function highlightCode(source: string, language: string): string {
  return highlightCodeLines(source, language).join("\n");
}
