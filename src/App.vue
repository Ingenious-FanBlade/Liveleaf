<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import type { ComponentPublicInstance } from "vue";
import { parseCourse } from "./markdown/parseCourse";

const starterSource = `# Python Basics

::: note
Stream Learn files stay readable Markdown first, then add teaching blocks where plain Markdown runs out of room.
:::

\`\`\`python run id="hello-stream" timeout=5000
name = "Stream Learn"
print(f"Hello, {name}")
\`\`\`

---

# Variables

::: columns
::: column width=45%
## Concept

A variable stores a value so the program can use it later.
:::

::: column width=55%
\`\`\`python exercise id="change-name"
name = "Ada"
print(name)
\`\`\`
:::
:::

---

# Media And Reveals

::video[Sorting demo](./assets/sort.mp4){controls start=12 width=80%}

::: hint
Think about what state needs to be remembered between two lines of code.
:::

::: solution collapsed
Use a dictionary to store previously seen values.
:::
`;

const source = ref(starterSource);
const isNavigatorOpen = ref(true);
const activeSlideId = ref("slide-1");
const previewStageRef = ref<HTMLElement | null>(null);
const slideRefs = new Map<string, HTMLElement>();

const course = computed(() => parseCourse(source.value));

type TemplateRef = Element | ComponentPublicInstance | null;

watch(
  () => course.value.slides.map((slide) => slide.id).join(","),
  () => {
    if (!course.value.slides.some((slide) => slide.id === activeSlideId.value)) {
      activeSlideId.value = course.value.slides[0]?.id ?? "slide-1";
    }
  },
);

function setSlideRef(slideId: string, element: TemplateRef) {
  if (element instanceof HTMLElement) {
    slideRefs.set(slideId, element);
    return;
  }

  if (element === null) {
    slideRefs.delete(slideId);
  }
}

function selectSlide(slideId: string) {
  activeSlideId.value = slideId;

  nextTick(() => {
    const target = slideRefs.get(slideId);
    if (!target) {
      return;
    }

    const behavior: ScrollBehavior = window.matchMedia(
      "(prefers-reduced-motion: reduce)",
    ).matches
      ? "auto"
      : "smooth";

    target.scrollIntoView({ behavior, block: "start", inline: "nearest" });
  });
}

function toggleNavigator() {
  isNavigatorOpen.value = !isNavigatorOpen.value;
}

function updateActiveSlideFromScroll() {
  const stage = previewStageRef.value;
  if (!stage) {
    return;
  }

  const stageRect = stage.getBoundingClientRect();
  const readingLine = stageRect.top + stage.clientHeight * 0.38;
  let closestSlideId = activeSlideId.value;
  let closestDistance = Number.POSITIVE_INFINITY;

  for (const slide of course.value.slides) {
    const slideElement = slideRefs.get(slide.id);
    if (!slideElement) {
      continue;
    }

    const slideRect = slideElement.getBoundingClientRect();
    const distance = Math.abs(slideRect.top - readingLine);

    if (distance < closestDistance) {
      closestDistance = distance;
      closestSlideId = slide.id;
    }
  }

  activeSlideId.value = closestSlideId;
}
</script>

<template>
  <main
    class="course-shell paper-bg"
    :class="{ 'navigator-collapsed': !isNavigatorOpen }"
    aria-label="Stream Learn markdown workspace"
  >
    <header class="top-rule">
      <div class="navigator-head">
        <button
          class="fold-button"
          type="button"
          :aria-label="isNavigatorOpen ? '收起页面浏览栏' : '展开页面浏览栏'"
          :title="isNavigatorOpen ? '收起页面浏览栏' : '展开页面浏览栏'"
          @click="toggleNavigator"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path v-if="isNavigatorOpen" d="m15 6-6 6 6 6" />
            <path v-else d="m9 6 6 6-6 6" />
          </svg>
        </button>

        <div v-if="isNavigatorOpen" class="navigator-title">
          <span>Slides</span>
          <strong>{{ course.slides.length }}</strong>
        </div>
      </div>

      <div class="lesson-title">
        <span>Stream Learn</span>
        <strong>{{ course.title }}</strong>
      </div>

      <div class="lesson-stats" aria-label="Document stats">
        <span>{{ course.codeCells.length }} code</span>
        <span>{{ source.length }} chars</span>
      </div>
    </header>

    <div class="workspace-layout">
      <aside class="slide-navigator" aria-label="页面浏览栏">
        <div v-if="isNavigatorOpen" class="thumbnail-stack">
          <button
            v-for="slide in course.slides"
            :key="slide.id"
            class="slide-thumbnail"
            :class="{ active: slide.id === activeSlideId }"
            type="button"
            :aria-current="slide.id === activeSlideId ? 'page' : undefined"
            @click="selectSlide(slide.id)"
          >
            <span class="thumbnail-number">{{ slide.index + 1 }}</span>
            <span class="thumbnail-label">{{ slide.title }}</span>
            <span v-if="slide.codeCells.length" class="thumbnail-meta">
              {{ slide.codeCells.length }} code
            </span>
          </button>
        </div>

        <div v-else class="collapsed-slides" aria-label="页面快速选择">
          <button
            v-for="slide in course.slides"
            :key="slide.id"
            class="slide-dot"
            :class="{ active: slide.id === activeSlideId }"
            type="button"
            :aria-current="slide.id === activeSlideId ? 'page' : undefined"
            :aria-label="`打开 ${slide.title}`"
            @click="selectSlide(slide.id)"
          >
            {{ slide.index + 1 }}
          </button>
        </div>
      </aside>

      <section class="editor-pane" aria-labelledby="editor-title">
        <div class="pane-title">
          <span id="editor-title">Markdown Source</span>
          <strong>.sl.md</strong>
        </div>

        <textarea
          v-model="source"
          class="source-editor"
          spellcheck="false"
          aria-label="Markdown source editor"
        />
      </section>

      <section class="preview-pane" aria-labelledby="preview-title">
        <div class="pane-title">
          <span id="preview-title">Live Preview</span>
          <strong>{{ course.slides.length }} slides</strong>
        </div>

        <div
          ref="previewStageRef"
          class="preview-stage"
          aria-label="实时预览"
          @scroll.passive="updateActiveSlideFromScroll"
        >
          <article
            v-for="slide in course.slides"
            :key="slide.id"
            :ref="(element) => setSlideRef(slide.id, element)"
            class="preview-slide"
            :class="{ active: slide.id === activeSlideId }"
            :aria-labelledby="`preview-slide-title-${slide.id}`"
          >
            <div class="slide-meta">
              <span>Slide {{ slide.index + 1 }}</span>
              <strong v-if="slide.codeCells.length">
                {{ slide.codeCells.length }} code
              </strong>
            </div>

            <div
              :id="`preview-slide-title-${slide.id}`"
              class="markdown-preview"
              v-html="slide.html"
            />
          </article>
        </div>
      </section>
    </div>
  </main>
</template>

<style>
:root {
  --paper: #f8f1df;
  --paper-soft: #fbf7ea;
  --surface: #fffdf5;
  --ink: #334454;
  --ink-dark: #263747;
  --muted: #697174;
  --blue: #65b9e6;
  --red: #ef5549;
  --yellow: #f3b642;
  --green: #4e8b6f;
  --shadow: #2f4152;
  --line-soft: rgba(51, 68, 84, 0.08);
  color: var(--ink-dark);
  background: var(--paper);
  font-family: Nunito, Inter, "Segoe UI", system-ui, sans-serif;
  font-size: 16px;
  font-synthesis: none;
  line-height: 1.5;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

* {
  box-sizing: border-box;
}

body {
  min-width: 320px;
  min-height: 100vh;
  margin: 0;
  background: var(--paper);
}

button,
textarea {
  font: inherit;
}
</style>

<style scoped>
.course-shell {
  height: 100vh;
  min-height: 100vh;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  overflow: hidden;
}

.paper-bg {
  background-color: var(--paper);
  background-image:
    linear-gradient(rgba(51, 68, 84, 0.045) 1px, transparent 1px),
    linear-gradient(90deg, rgba(51, 68, 84, 0.045) 1px, transparent 1px);
  background-size: 24px 24px;
}

.top-rule {
  min-height: 86px;
  display: grid;
  grid-template-columns: 190px minmax(0, 1fr) auto;
  align-items: center;
  column-gap: 22px;
  padding: 14px 24px 14px 14px;
  border-bottom: 5px solid var(--ink);
  background: rgba(251, 247, 234, 0.78);
}

.navigator-head,
.lesson-title,
.lesson-stats,
.pane-title,
.slide-meta {
  display: flex;
  align-items: center;
}

.navigator-head {
  gap: 12px;
}

.fold-button {
  width: 48px;
  height: 48px;
  display: grid;
  place-items: center;
  border: 4px solid var(--ink);
  border-radius: 10px;
  background: var(--surface);
  color: var(--ink-dark);
  box-shadow: 6px 6px 0 var(--shadow);
  cursor: pointer;
  transition:
    transform 180ms cubic-bezier(0.2, 0.8, 0.2, 1),
    box-shadow 180ms cubic-bezier(0.2, 0.8, 0.2, 1);
}

.fold-button svg {
  width: 26px;
  height: 26px;
  fill: none;
  stroke: currentColor;
  stroke-width: 3.4;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.fold-button:hover,
.slide-thumbnail:hover,
.slide-thumbnail.active {
  transform: translate(-2px, -2px);
}

.fold-button:hover {
  box-shadow: 8px 8px 0 var(--shadow);
}

.fold-button:active {
  transform: translate(2px, 2px);
  box-shadow: 3px 3px 0 var(--shadow);
}

.fold-button:focus-visible,
.slide-thumbnail:focus-visible,
.slide-dot:focus-visible,
.source-editor:focus-visible {
  outline: 4px solid var(--blue);
  outline-offset: 4px;
}

.navigator-title {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--muted);
  font-weight: 900;
}

.navigator-title strong {
  min-width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border: 3px solid var(--ink);
  border-radius: 999px;
  background: var(--yellow);
  color: var(--ink-dark);
}

.lesson-title {
  min-width: 0;
  flex-wrap: wrap;
  gap: 12px;
  color: var(--muted);
  font-weight: 900;
}

.lesson-title span {
  color: var(--green);
}

.lesson-title strong {
  min-width: 0;
  color: var(--ink-dark);
  font-family: Georgia, "Times New Roman", serif;
  font-size: clamp(1.25rem, 2.1vw, 1.85rem);
  overflow-wrap: anywhere;
}

.lesson-stats {
  justify-content: flex-end;
  gap: 8px;
  white-space: nowrap;
}

.lesson-stats span,
.pane-title strong,
.slide-meta strong,
.thumbnail-meta {
  border: 3px solid var(--ink);
  border-radius: 999px;
  background: var(--yellow);
  color: var(--ink-dark);
  font-size: 0.82rem;
  font-weight: 900;
}

.lesson-stats span {
  padding: 5px 10px;
}

.workspace-layout {
  min-height: 0;
  display: grid;
  grid-template-columns: 190px minmax(300px, 0.92fr) minmax(360px, 1.12fr);
  gap: 22px;
  padding: 22px 24px 30px 14px;
  overflow: hidden;
}

.slide-navigator,
.editor-pane,
.preview-pane {
  min-width: 0;
  min-height: 0;
}

.slide-navigator {
  overflow-y: auto;
  padding: 0 12px 28px 0;
}

.thumbnail-stack {
  display: grid;
  gap: 16px;
  justify-items: end;
}

.slide-thumbnail {
  position: relative;
  width: min(152px, 100%);
  min-height: 116px;
  display: grid;
  align-content: center;
  gap: 7px;
  padding: 18px 14px;
  border: 4px solid var(--ink);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-dark);
  box-shadow: 8px 8px 0 rgba(47, 65, 82, 0.16);
  cursor: pointer;
  text-align: left;
  transition:
    transform 180ms cubic-bezier(0.2, 0.8, 0.2, 1),
    box-shadow 180ms cubic-bezier(0.2, 0.8, 0.2, 1),
    background 180ms cubic-bezier(0.2, 0.8, 0.2, 1);
}

.slide-thumbnail::before {
  position: absolute;
  top: 11px;
  left: 12px;
  width: 28px;
  height: 8px;
  border-radius: 999px;
  background: var(--blue);
  content: "";
}

.slide-thumbnail:nth-child(3n + 2)::before {
  background: var(--green);
}

.slide-thumbnail:nth-child(3n)::before {
  background: var(--red);
}

.slide-thumbnail:hover,
.slide-thumbnail.active {
  box-shadow: 10px 10px 0 var(--shadow);
}

.slide-thumbnail.active {
  background: var(--paper-soft);
}

.thumbnail-number {
  position: absolute;
  right: 12px;
  bottom: 7px;
  color: rgba(51, 68, 84, 0.24);
  font-family: Georgia, "Times New Roman", serif;
  font-size: 2rem;
  font-weight: 900;
}

.thumbnail-label {
  position: relative;
  z-index: 1;
  color: var(--ink-dark);
  font-weight: 900;
  line-height: 1.18;
  overflow-wrap: anywhere;
}

.thumbnail-meta {
  width: max-content;
  padding: 3px 8px;
  background: var(--paper);
}

.collapsed-slides {
  width: 54px;
  display: grid;
  gap: 12px;
  justify-items: center;
}

.slide-dot {
  width: 42px;
  height: 42px;
  display: grid;
  place-items: center;
  border: 3px solid var(--ink);
  border-radius: 999px;
  background: var(--surface);
  color: var(--ink-dark);
  box-shadow: 4px 4px 0 rgba(47, 65, 82, 0.22);
  cursor: pointer;
  font-weight: 900;
}

.slide-dot.active {
  background: var(--yellow);
  box-shadow: 5px 5px 0 var(--shadow);
}

.editor-pane,
.preview-pane {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 12px;
}

.pane-title {
  justify-content: space-between;
  gap: 12px;
  min-height: 38px;
  color: var(--green);
  font-weight: 900;
}

.pane-title strong,
.slide-meta strong {
  padding: 4px 9px;
}

.source-editor {
  width: 100%;
  height: 100%;
  min-height: 0;
  resize: none;
  padding: 20px;
  border: 4px solid var(--ink);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: 12px 12px 0 rgba(47, 65, 82, 0.16);
  color: var(--ink-dark);
  font-family: "JetBrains Mono", "SFMono-Regular", Consolas, monospace;
  font-size: 0.94rem;
  line-height: 1.62;
  outline: none;
  tab-size: 2;
}

.preview-stage {
  min-height: 0;
  overflow-y: auto;
  scroll-padding-top: 4px;
  padding: 0 16px 30px 0;
}

.preview-slide {
  margin: 0 0 24px;
  padding: clamp(22px, 4vw, 42px);
  border: 4px solid var(--ink);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: 14px 14px 0 rgba(47, 65, 82, 0.18);
}

.preview-slide.active {
  box-shadow: 16px 16px 0 rgba(47, 65, 82, 0.26);
}

.slide-meta {
  justify-content: space-between;
  gap: 12px;
  margin-bottom: clamp(24px, 4vw, 40px);
  color: var(--muted);
  font-weight: 900;
}

.slide-meta span {
  color: var(--green);
}

.markdown-preview {
  color: var(--ink);
  font-weight: 750;
}

.markdown-preview :deep(*) {
  overflow-wrap: anywhere;
}

.markdown-preview :deep(h1),
.markdown-preview :deep(h2),
.markdown-preview :deep(h3) {
  margin: 0 0 14px;
  color: var(--ink-dark);
  font-family: Fraunces, "Libre Baskerville", Georgia, serif;
  line-height: 1.05;
  letter-spacing: 0;
}

.markdown-preview :deep(h1) {
  color: var(--red);
  font-size: clamp(2.35rem, 5vw, 4.6rem);
  text-shadow: 3px 3px 0 var(--ink);
}

.markdown-preview :deep(h2) {
  margin-top: 24px;
  font-size: clamp(1.5rem, 3vw, 2.2rem);
}

.markdown-preview :deep(p),
.markdown-preview :deep(ul),
.markdown-preview :deep(ol),
.markdown-preview :deep(blockquote) {
  margin: 0 0 16px;
}

.markdown-preview :deep(a) {
  color: #206b91;
  font-weight: 900;
}

.markdown-preview :deep(table) {
  width: 100%;
  margin: 18px 0;
  border-collapse: collapse;
  background: var(--paper-soft);
}

.markdown-preview :deep(th),
.markdown-preview :deep(td) {
  padding: 9px 11px;
  border: 3px solid var(--ink);
  text-align: left;
}

.markdown-preview :deep(th) {
  background: var(--yellow);
  color: var(--ink-dark);
}

.markdown-preview :deep(blockquote) {
  padding: 12px 16px;
  border-left: 6px solid var(--green);
  background: var(--paper-soft);
}

.markdown-preview :deep(:not(.sl-code-cell) > pre) {
  margin: 18px 0;
  padding: 18px;
  overflow: auto;
  border: 4px solid var(--ink);
  border-radius: 12px;
  background: #334a5e;
  color: var(--surface);
  box-shadow: 8px 8px 0 var(--shadow);
}

.markdown-preview :deep(code) {
  font-family: "JetBrains Mono", "SFMono-Regular", Consolas, monospace;
}

.markdown-preview :deep(:not(pre) > code) {
  padding: 2px 6px;
  border: 2px solid rgba(51, 68, 84, 0.35);
  border-radius: 6px;
  background: var(--paper-soft);
  color: var(--ink-dark);
  font-size: 0.9em;
}

.markdown-preview :deep(.sl-columns) {
  display: flex;
  flex-wrap: nowrap;
  align-items: stretch;
  gap: 0;
  margin: 18px 0;
  overflow: hidden;
}

.markdown-preview :deep(.sl-column) {
  flex: 1 1 var(--column-width, 0);
  min-width: 0;
  padding: 0 16px;
}

.markdown-preview :deep(.sl-column:first-child) {
  padding-left: 0;
}

.markdown-preview :deep(.sl-column + .sl-column) {
  border-left: 3px solid var(--ink);
}

.markdown-preview :deep(.sl-column:last-child) {
  padding-right: 0;
}

.markdown-preview :deep(.sl-column > :first-child) {
  margin-top: 0;
}

.markdown-preview :deep(.sl-column > :last-child) {
  margin-bottom: 0;
}

.markdown-preview :deep(.sl-callout),
.markdown-preview :deep(.sl-reveal) {
  margin: 18px 0;
  padding: 16px 18px;
  border: 4px solid var(--ink);
  border-radius: 12px;
  background: var(--paper-soft);
  box-shadow: 8px 8px 0 rgba(47, 65, 82, 0.14);
}

.markdown-preview :deep(.sl-callout--tip),
.markdown-preview :deep(.sl-reveal--hint) {
  border-color: var(--green);
}

.markdown-preview :deep(.sl-callout--warning),
.markdown-preview :deep(.sl-callout--danger),
.markdown-preview :deep(.sl-reveal--solution) {
  border-color: var(--red);
}

.markdown-preview :deep(.sl-callout__label),
.markdown-preview :deep(.sl-reveal summary) {
  width: max-content;
  margin-bottom: 10px;
  padding: 4px 10px;
  border: 3px solid var(--ink);
  border-radius: 999px;
  background: var(--yellow);
  color: var(--ink-dark);
  cursor: pointer;
  font-weight: 900;
}

.markdown-preview :deep(.sl-reveal summary) {
  margin-bottom: 0;
}

.markdown-preview :deep(.sl-reveal[open] summary) {
  margin-bottom: 10px;
}

.markdown-preview :deep(.sl-code-cell) {
  margin: 20px 0;
  overflow: hidden;
  border: 4px solid var(--ink);
  border-radius: 14px;
  background: #334a5e;
  color: var(--surface);
  box-shadow: 9px 9px 0 var(--shadow);
}

.markdown-preview :deep(.sl-code-cell__header) {
  min-height: 44px;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 8px 12px 8px 16px;
  border-bottom: 3px solid var(--ink);
  background: var(--blue);
  color: var(--ink-dark);
  font-family: "JetBrains Mono", "SFMono-Regular", Consolas, monospace;
  font-size: 0.82rem;
  font-weight: 900;
}

.markdown-preview :deep(.sl-code-cell__header div) {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
}

.markdown-preview :deep(.sl-code-cell__header span) {
  padding: 2px 7px;
  border: 2px solid var(--ink);
  border-radius: 999px;
  background: var(--paper-soft);
  color: var(--ink-dark);
}

.markdown-preview :deep(.sl-code-cell pre) {
  margin: 0;
  padding: 20px;
  overflow: auto;
  background: transparent;
  color: var(--surface);
  line-height: 1.68;
}

.markdown-preview :deep(.sl-code-cell--exercise .sl-code-cell__header) {
  background: var(--green);
  color: var(--surface);
}

.markdown-preview :deep(.sl-media) {
  width: min(var(--media-width, 100%), 100%);
  margin: 20px 0;
}

.markdown-preview :deep(.sl-media video),
.markdown-preview :deep(.sl-media audio) {
  width: 100%;
  border: 4px solid var(--ink);
  border-radius: 12px;
  background: var(--paper-soft);
  box-shadow: 8px 8px 0 rgba(47, 65, 82, 0.16);
}

.markdown-preview :deep(.sl-media figcaption) {
  margin-top: 10px;
  color: var(--muted);
  font-weight: 900;
}

.navigator-collapsed .top-rule {
  grid-template-columns: 58px minmax(0, 1fr) auto;
}

.navigator-collapsed .workspace-layout {
  grid-template-columns: 58px minmax(300px, 0.92fr) minmax(360px, 1.12fr);
}

.navigator-collapsed .slide-navigator {
  padding-right: 0;
}

@media (max-width: 1120px) {
  .workspace-layout,
  .navigator-collapsed .workspace-layout {
    grid-template-columns: 150px minmax(0, 1fr);
    grid-template-rows: minmax(360px, 0.88fr) minmax(420px, 1.12fr);
  }

  .slide-navigator {
    grid-row: 1 / span 2;
  }

  .editor-pane,
  .preview-pane {
    min-height: 0;
  }
}

@media (max-width: 780px) {
  .course-shell {
    overflow: auto;
  }

  .top-rule {
    grid-template-columns: 1fr;
    gap: 12px;
    padding-right: 16px;
  }

  .lesson-stats {
    justify-content: flex-start;
  }

  .workspace-layout,
  .navigator-collapsed .workspace-layout {
    grid-template-columns: 1fr;
    grid-template-rows: auto minmax(420px, 54vh) minmax(420px, auto);
    padding: 18px;
    overflow: visible;
  }

  .slide-navigator {
    grid-row: auto;
    overflow: visible;
    padding: 0;
  }

  .thumbnail-stack {
    grid-template-columns: repeat(3, minmax(0, 1fr));
    justify-items: stretch;
    gap: 14px;
  }

  .slide-thumbnail {
    width: 100%;
    min-height: 106px;
  }

  .collapsed-slides {
    width: auto;
    grid-auto-flow: column;
    justify-content: start;
  }

  .preview-stage {
    overflow: visible;
    padding-right: 0;
  }
}

@media (max-width: 560px) {
  .top-rule {
    min-height: 82px;
    border-bottom-width: 4px;
  }

  .workspace-layout,
  .navigator-collapsed .workspace-layout {
    padding: 14px;
  }

  .thumbnail-stack {
    grid-template-columns: 1fr;
  }

  .slide-thumbnail {
    min-height: 84px;
  }

  .preview-slide {
    padding: 22px 16px;
    border-width: 3px;
    box-shadow: 10px 10px 0 rgba(47, 65, 82, 0.18);
  }

  .markdown-preview :deep(h1) {
    font-size: clamp(2.3rem, 15vw, 3.6rem);
    text-shadow: 3px 3px 0 var(--ink);
  }
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    scroll-behavior: auto !important;
    transition-duration: 0.01ms !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
  }
}
</style>
