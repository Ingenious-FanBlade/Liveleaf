<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { EditorState } from "@codemirror/state";
import { EditorView, drawSelection, highlightActiveLine, highlightActiveLineGutter, keymap, lineNumbers } from "@codemirror/view";
import { bracketMatching, indentOnInput, indentUnit } from "@codemirror/language";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { codeSyntaxHighlighting, courseMarkdown } from "./syntaxHighlight";

const props = defineProps<{ modelValue: string }>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();
const container = ref<HTMLElement>();
let view: EditorView | undefined;

function createState(doc: string) {
  return EditorState.create({
      doc,
      extensions: [
        courseMarkdown,
        codeSyntaxHighlighting,
        lineNumbers(),
        history(),
        drawSelection(),
        highlightActiveLine(),
        highlightActiveLineGutter(),
        bracketMatching(),
        indentOnInput(),
        indentUnit.of("    "),
        EditorState.tabSize.of(4),
        EditorView.lineWrapping,
        keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
        EditorView.contentAttributes.of({
          "aria-label": "Markdown source editor",
          spellcheck: "false",
          autocapitalize: "off",
          autocorrect: "off",
        }),
        EditorView.updateListener.of(update => {
          if (update.docChanged) {
            const value = update.state.doc.toString();
            if (value !== props.modelValue) emit("update:modelValue", value);
          }
        }),
        EditorView.theme({
          "&": { height: "100%", backgroundColor: "var(--code-background)", color: "var(--code-foreground)" },
          "&.cm-focused": { outline: "none" },
          ".cm-scroller": { overflow: "auto", fontFamily: "'JetBrains Mono', 'SFMono-Regular', Consolas, monospace", lineHeight: "1.62" },
          ".cm-content": { padding: "16px 0", caretColor: "#fffdf5" },
          ".cm-line": { padding: "0 16px 0 10px" },
          ".cm-gutters": { backgroundColor: "#2d4255", color: "#b4c4cf", borderRight: "2px solid #263747", paddingRight: "6px" },
          ".cm-activeLineGutter": { backgroundColor: "#46576a", color: "#fffdf5" },
          ".cm-activeLine": { backgroundColor: "#ffffff09" },
          ".cm-cursor, .cm-dropCursor": { borderLeftColor: "#fffdf5" },
          "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": { backgroundColor: "#28769b" },
          ".cm-matchingBracket": { backgroundColor: "#ffffff24", outline: "1px solid #a5def5" },
        }, { dark: true }),
      ],
  });
}

onMounted(() => {
  if (!container.value) return;
  view = new EditorView({ parent: container.value, state: createState(props.modelValue) });
});

watch(() => props.modelValue, value => {
  if (!view || view.state.doc.toString() === value) return;
  // A document loaded externally gets its own undo history and cursor position.
  view.setState(createState(value));
});

onBeforeUnmount(() => { view?.destroy(); view = undefined; });
</script>

<template>
  <div ref="container" class="source-editor sl-code-theme" />
</template>

<style scoped>
.source-editor {
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  border: 4px solid var(--ink, #334454);
  border-radius: 8px;
  background: #334a5e;
  box-shadow: 12px 12px 0 #2f415229;
  font-size: .94rem;
}
.source-editor:focus-within {
  outline: 4px solid var(--blue, #65b9e6);
  outline-offset: 4px;
}
</style>
