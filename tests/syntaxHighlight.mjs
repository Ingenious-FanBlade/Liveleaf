import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { EditorState } from "@codemirror/state";
import { ensureSyntaxTree } from "@codemirror/language";
import { highlightTree } from "@lezer/highlight";
import { createServer } from "vite";

const server = await createServer({ server: { middlewareMode: true }, appType: "custom" });
try {
  const { highlightCode, highlightCodeLines, codeLanguage, codeHighlighter, codeSyntaxHighlighting, courseMarkdown } = await server.ssrLoadModule("/src/code/syntaxHighlight.ts");
  const { parseCourse } = await server.ssrLoadModule("/src/markdown/parseCourse.ts");
  const code = "def preorder(node):\n    if node is None:\n        return\n    preorder(node.left)\n    node.visit()\n";
  const html = highlightCode(code, "python");
  assert.match(html, /class="sl-syntax-keyword">def<\/span>/);
  assert.match(html, /class="sl-syntax-variable">node<\/span>/);
  assert.equal((html.match(/class="sl-syntax-function">preorder<\/span>/g) ?? []).length, 2, "Definitions and calls use the function color");
  assert.match(html, /class="sl-syntax-function">visit<\/span>/);
  assert.equal(highlightCode(code, "py"), html);
  assert.equal(highlightCode(code, "PYTHON run watch=root"), html);

  // Editor Markdown must mount the same Python parser even with runnable metadata.
  const markdown = `# Test\n\n\`\`\`python run id=tree visualize=tree watch=root\n${code}\`\`\`\n`;
  const state = EditorState.create({ doc: markdown, extensions: [courseMarkdown, codeSyntaxHighlighting] });
  const tree = ensureSyntaxTree(state, state.doc.length, 1000);
  assert.ok(tree);
  const offset = markdown.indexOf(code);
  const editorTokens = [];
  highlightTree(tree, codeHighlighter, (from, to, classes) => {
    if (from >= offset && to <= offset + code.length) editorTokens.push([from - offset, to - offset, classes]);
  });
  const staticTokens = [];
  highlightTree(codeLanguage("python").parser.parse(code), codeHighlighter, (from, to, classes) => staticTokens.push([from, to, classes]));
  assert.deepEqual(editorTokens, staticTokens, "Editor and read-only view must highlight every Python token identically");

  for (const kind of ["", " run id=example", " exercise id=example"]) {
    const course = parseCourse(`\`\`\`python${kind}\n${code}\`\`\``);
    assert.ok(course.slides[0].html.includes(html));
    assert.match(course.slides[0].html, /<pre class="sl-code-theme">/);
    if (kind) assert.equal(course.codeCells[0].code, code, "Highlighting must preserve executable source");
  }

  const multiline = 'message = """first\nif return <script>\nlast"""\n';
  const lines = highlightCodeLines(multiline, "python");
  assert.equal(lines.length, multiline.split("\n").length);
  assert.match(lines[1], /^<span class="sl-syntax-string">if return &lt;script&gt;<\/span>$/);
  for (const line of lines) assert.equal((line.match(/<span /g) ?? []).length, (line.match(/<\/span>/g) ?? []).length, "Each replay line must contain balanced spans");
  assert.ok(!highlightCode('message = "if return def" # def fake(x)', "python").includes('class="sl-syntax-keyword"'), "Strings and comments must not be treated as code");
  assert.match(highlightCode('message = f"Hello {name}"', "python"), /class="sl-syntax-variable">name<\/span>/);
  assert.match(highlightCode('const value = 1; function visit(node) { return node; } visit(value);', "js"), /class="sl-syntax-function">visit<\/span>/);
  assert.match(highlightCode('function visit(node: string): string { return node; }', "ts"), /class="sl-syntax-keyword">function<\/span>/);
  assert.match(highlightCode('{"value": 1}', "json"), /sl-syntax-literal/);
  assert.ok(highlightCode("def unfinished(", "python").includes("sl-syntax-keyword"), "Incomplete edits remain highlightable");
  const unsafe = '<script>alert("x")</script> & <img src=x onerror=alert(1)>';
  assert.ok(!highlightCode(unsafe, "python").includes("<script>"));
  assert.equal(highlightCode(unsafe, "unknown"), '&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt; &amp; &lt;img src=x onerror=alert(1)&gt;');
  const example = await readFile(new URL("../docs/examples/python-visualization.ll.md", import.meta.url), "utf8");
  assert.match(parseCourse(example).slides.at(-1).html, /sl-syntax-function">preorder<\/span>/);
  console.log("Passed syntax highlighting checks: editor/preview parity, executable cells, multiline replay, aliases, incomplete code and HTML escaping.");
} finally { await server.close(); }
