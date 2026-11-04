<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { ComponentPublicInstance } from "vue";
import { parseCourse } from "./markdown/parseCourse";
import type { CodeCell, CourseSlide } from "./markdown/documentModel";
import { openProjectPackage, saveProjectPackage } from "./project/projectPackageClient";
import type {
  OpenProjectPackageResponse,
  SaveProjectPackageResponse,
} from "./project/projectPackageClient";
import { PythonRuntimeClient } from "./runtime/PythonRuntimeClient";
import type { RunResult } from "./runtime/runResult";
import { readVisualization } from "./runtime/visualization";
import type { VisualizationTrace } from "./runtime/visualization";
import TracePlayer from "./visualization/TracePlayer.vue";
import CodeEditor from "./code/CodeEditor.vue";
import WindowFrame from "./window/WindowFrame.vue";
import visualizationExamples from "../docs/examples/python-visualization.ll.md?raw";
import dijkstraExamples from "../docs/examples/dijkstra.ll.md?raw";

const starterSource = `# Python Basics

::: note
Liveleaf files stay readable Markdown first, then add teaching blocks where plain Markdown runs out of room.
:::

\`\`\`python run id="hello-liveleaf" timeout=5000
name = "Liveleaf"
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
` + "\n---\n\n" + visualizationExamples + "\n---\n\n" + dijkstraExamples;

const source = ref(starterSource);
const isNavigatorOpen = ref(true);
const activeSlideId = ref("slide-1");
const previewStageRef = ref<HTMLElement | null>(null);
const presentationRootRef = ref<HTMLElement | null>(null);
const windowFrameRef = ref<InstanceType<typeof WindowFrame> | null>(null);
const presentationScrollRef = ref<HTMLElement | null>(null);
const isPresentationMode = ref(false);
const slideRefs = new Map<string, HTMLElement>();
const pythonRuntime = new PythonRuntimeClient();
const currentSourceFilePath = ref<string | undefined>();
const currentWorkingDirectory = ref<string | undefined>();
const currentPackagePath = ref<string | undefined>();
const packageResourceUrls = ref<Record<string, string>>({});

const course = computed(() =>
  parseCourse(source.value, {
    resolveResourceUrl: resolvePreviewResourceUrl,
  }),
);

const activeSlideIndex = computed(() =>
  course.value.slides.findIndex((slide) => slide.id === activeSlideId.value),
);
const activeSlide = computed(
  () => course.value.slides[activeSlideIndex.value] ?? course.value.slides[0],
);
const isFirstSlide = computed(() => Math.max(activeSlideIndex.value, 0) <= 0);
const isLastSlide = computed(() => {
  const lastIndex = course.value.slides.length - 1;

  return lastIndex <= 0 || Math.max(activeSlideIndex.value, 0) >= lastIndex;
});
const presentationSlideLabel = computed(() => {
  const slideCount = course.value.slides.length;
  const slideNumber = slideCount === 0 ? 0 : Math.max(activeSlideIndex.value, 0) + 1;

  return `${slideNumber} / ${slideCount}`;
});

type TemplateRef = Element | ComponentPublicInstance | null;
type CellRunStatus = RunResult["status"] | "pending";
type ProjectSaveStatusKind = "idle" | "opening" | "saving" | "success" | "error";

interface CellRunState {
  status: CellRunStatus;
  result?: RunResult;
  message?: string;
  sourceSnapshot?: string;
  documentLine?: number;
  stale?: boolean;
  trace?: VisualizationTrace;
  visualizationError?: string;
}

interface ProjectSaveStatus {
  kind: ProjectSaveStatusKind;
  message?: string;
  warnings?: string[];
}

const runStates = ref<Record<string, CellRunState>>({});
const requestSequences = new Map<string, number>();
const isOpeningProject = ref(false);
const isSavingProject = ref(false);
const projectSaveStatus = ref<ProjectSaveStatus>({ kind: "idle" });
const isProjectBusy = computed(() => isOpeningProject.value || isSavingProject.value);

watch(
  () => course.value.slides.map((slide) => slide.id).join(","),
  () => {
    if (!course.value.slides.some((slide) => slide.id === activeSlideId.value)) {
      activeSlideId.value = course.value.slides[0]?.id ?? "slide-1";
    }
  },
);

watch(source, () => {
  for (const state of Object.values(runStates.value)) state.stale = true;

  if (projectSaveStatus.value.kind === "success") {
    projectSaveStatus.value = { kind: "idle" };
  }
});

onMounted(() => {
  document.addEventListener("fullscreenchange", handleFullscreenChange);
  window.addEventListener("keydown", handlePresentationKeydown);
});

onBeforeUnmount(() => {
  document.removeEventListener("fullscreenchange", handleFullscreenChange);
  window.removeEventListener("keydown", handlePresentationKeydown);
});

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

    const behavior: ScrollBehavior = prefersReducedMotion() ? "auto" : "smooth";

    target.scrollIntoView({ behavior, block: "start", inline: "nearest" });
  });
}

function toggleNavigator() {
  isNavigatorOpen.value = !isNavigatorOpen.value;
}

async function enterPresentation() {
  if (course.value.slides.length === 0) {
    return;
  }

  isPresentationMode.value = true;
  await nextTick();
  resetPresentationScroll("auto");
  presentationRootRef.value?.focus({ preventScroll: true });

  const frame = windowFrameRef.value?.element;
  if (!document.fullscreenElement && frame?.requestFullscreen) {
    try {
      await frame.requestFullscreen();
    } catch {
      // Keep the in-app presentation layer when system fullscreen is unavailable.
    }
  }
}

async function exitPresentation() {
  const fullscreenElement = document.fullscreenElement;

  if (fullscreenElement && fullscreenElement === windowFrameRef.value?.element) {
    try {
      await document.exitFullscreen();
    } catch {
      // The local presentation state still needs to leave presentation mode.
    }
  }

  isPresentationMode.value = false;
  syncPreviewToActiveSlide();
}

function goToNextSlide() {
  const slides = course.value.slides;
  const currentIndex = Math.max(activeSlideIndex.value, 0);
  const nextSlide = slides[currentIndex + 1];

  if (!nextSlide) {
    return;
  }

  activeSlideId.value = nextSlide.id;
  resetPresentationScroll("auto");
}

function goToPreviousSlide() {
  const slides = course.value.slides;
  const currentIndex = Math.max(activeSlideIndex.value, 0);
  const previousSlide = slides[currentIndex - 1];

  if (!previousSlide) {
    return;
  }

  activeSlideId.value = previousSlide.id;
  resetPresentationScroll("auto");
}

function scrollPresentationSlide(direction: 1 | -1) {
  const stage = presentationScrollRef.value;

  if (!stage) {
    return;
  }

  const behavior: ScrollBehavior = prefersReducedMotion() ? "auto" : "smooth";
  const distance = Math.max(160, stage.clientHeight * 0.72);

  stage.scrollBy({
    top: direction * distance,
    behavior,
  });
}

function resetPresentationScroll(behavior: ScrollBehavior = "auto") {
  nextTick(() => {
    presentationScrollRef.value?.scrollTo({ top: 0, behavior });
  });
}

function syncPreviewToActiveSlide() {
  nextTick(() => {
    const target = slideRefs.get(activeSlideId.value);

    target?.scrollIntoView({ behavior: "auto", block: "start", inline: "nearest" });
  });
}

function handleFullscreenChange() {
  if (isPresentationMode.value && !document.fullscreenElement) {
    isPresentationMode.value = false;
    syncPreviewToActiveSlide();
  }
}

function handlePresentationKeydown(event: KeyboardEvent) {
  if (!isPresentationMode.value || shouldIgnorePresentationShortcut(event)) {
    return;
  }

  if (event.shiftKey && event.key === "ArrowDown") {
    event.preventDefault();
    scrollPresentationSlide(1);
    return;
  }

  if (event.shiftKey && event.key === "ArrowUp") {
    event.preventDefault();
    scrollPresentationSlide(-1);
    return;
  }

  if (event.shiftKey) {
    return;
  }

  switch (event.key) {
    case "ArrowRight":
    case "ArrowDown":
    case "PageDown":
    case " ":
    case "Spacebar":
    case "Enter":
      event.preventDefault();
      goToNextSlide();
      break;
    case "ArrowLeft":
    case "ArrowUp":
    case "PageUp":
    case "Backspace":
      event.preventDefault();
      goToPreviousSlide();
      break;
    case "Escape":
      event.preventDefault();
      void exitPresentation();
      break;
  }
}

function shouldIgnorePresentationShortcut(event: KeyboardEvent) {
  if (event.defaultPrevented || event.ctrlKey || event.altKey || event.metaKey) {
    return true;
  }

  if (!(event.target instanceof Element)) {
    return false;
  }

  const interactiveTarget = event.target.closest(
    "button, a, summary, input, textarea, select, option, video, audio, [role='button']",
  );
  const editableTarget = event.target.closest<HTMLElement>("[contenteditable]");

  return Boolean(
    interactiveTarget ||
      (editableTarget && editableTarget.getAttribute("contenteditable") !== "false"),
  );
}

function prefersReducedMotion() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

async function handleOpenProject() {
  if (isProjectBusy.value) {
    return;
  }

  if (!isTauriRuntime()) {
    projectSaveStatus.value = {
      kind: "error",
      message: "Project packages can be opened in the Tauri desktop app.",
    };
    return;
  }

  try {
    const selectedPath = await open({
      title: "Open Liveleaf Project",
      multiple: false,
      filters: [
        {
          name: "Liveleaf Project",
          // .slpkg: packages saved before the rename to Liveleaf.
          extensions: ["llpkg", "slpkg"],
        },
      ],
    });
    const packagePath = Array.isArray(selectedPath) ? selectedPath[0] : selectedPath;

    if (!packagePath) {
      return;
    }

    isOpeningProject.value = true;
    projectSaveStatus.value = {
      kind: "opening",
      message: "Opening project package...",
    };

    const result = await openProjectPackage({ packagePath });
    loadOpenedProject(result);
  } catch (error) {
    projectSaveStatus.value = {
      kind: "error",
      message: errorMessage(error),
    };
  } finally {
    isOpeningProject.value = false;
  }
}

async function handleSaveProject() {
  if (isProjectBusy.value) {
    return;
  }

  if (!isTauriRuntime()) {
    projectSaveStatus.value = {
      kind: "error",
      message: "Project packages can be saved in the Tauri desktop app.",
    };
    return;
  }

  try {
    const selectedPath = await save({
      title: "Save Liveleaf Project",
      defaultPath: `${sanitizeProjectFileName(course.value.title)}.llpkg`,
      filters: [
        {
          name: "Liveleaf Project",
          extensions: ["llpkg"],
        },
      ],
    });

    if (!selectedPath) {
      return;
    }

    isSavingProject.value = true;
    projectSaveStatus.value = {
      kind: "saving",
      message: "Saving project package...",
    };

    const result = await saveProjectPackage({
      source: source.value,
      outputPath: withProjectPackageExtension(selectedPath),
      sourceFilePath: currentSourceFilePath.value,
    });

    projectSaveStatus.value = {
      kind: "success",
      message: saveProjectSummary(result),
      warnings: result.warnings,
    };
  } catch (error) {
    projectSaveStatus.value = {
      kind: "error",
      message: errorMessage(error),
    };
  } finally {
    isSavingProject.value = false;
  }
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

function handlePreviewClick(event: MouseEvent) {
  handleCourseInteractionClick(event, previewStageRef.value);
}

function handlePresentationClick(event: MouseEvent) {
  handleCourseInteractionClick(event, presentationRootRef.value);
}

function handleCourseInteractionClick(event: MouseEvent, container: HTMLElement | null) {
  if (!(event.target instanceof Element)) {
    return;
  }

  const runButton = event.target.closest<HTMLButtonElement>("[data-code-cell-run]");
  const cellId = runButton?.dataset.codeCellRun;

  if (!runButton || !cellId || !container?.contains(runButton)) {
    return;
  }

  const cell = course.value.codeCells.find((candidate) => candidate.id === cellId);

  if (!cell) {
    return;
  }

  void runCodeCell(cell);
}

async function runCodeCell(cell: CodeCell) {
  if (cell.language.toLowerCase() !== "python") {
    setRunState(cell.id, {
      status: "error",
      message: `No runtime is configured for ${cell.language}.`,
    });
    return;
  }

  const sequence = (requestSequences.get(cell.id) ?? 0) + 1;
  requestSequences.set(cell.id, sequence);
  const sourceAtRun = source.value;
  const snapshot = { sourceSnapshot: cell.code, documentLine: cell.sourceRange?.startLine };
  setRunState(cell.id, { status: "pending", ...snapshot });

  try {
    const result = await pythonRuntime.runCodeCell({
      source: sourceAtRun,
      cellId: cell.id,
      filePath: currentSourceFilePath.value,
      workingDirectory: currentWorkingDirectory.value,
    });

    if (requestSequences.get(cell.id) !== sequence) return;
    const visualization = readVisualization(result.outputs, cell.id);
    setRunState(cell.id, {
      status: result.status,
      result,
      ...snapshot,
      stale: source.value !== sourceAtRun,
      trace: visualization.trace,
      visualizationError: visualization.error,
    });
  } catch (error) {
    if (requestSequences.get(cell.id) !== sequence) return;
    setRunState(cell.id, {
      status: "error",
      message: errorMessage(error),
    });
  }
}

function setRunState(cellId: string, state: CellRunState) {
  runStates.value = {
    ...runStates.value,
    [cellId]: state,
  };
}

function visibleRunCells(slide: CourseSlide) {
  return slide.codeCells.filter((cell) => runStates.value[cell.id]);
}

function runStateFor(cellId: string) {
  return runStates.value[cellId];
}

function runtimeOutputTarget(cellId: string) {
  return `[data-code-cell-output="${encodeURIComponent(cellId)}"]`;
}

function statusLabel(status: CellRunStatus | undefined) {
  switch (status) {
    case "pending":
      return "Running";
    case "success":
      return "Success";
    case "error":
      return "Error";
    case "timeout":
      return "Timeout";
    case "cancelled":
      return "Cancelled";
    default:
      return "Ready";
  }
}

function formatDuration(result: RunResult | undefined) {
  return result ? `${result.durationMs} ms` : "";
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function isTauriRuntime() {
  return "__TAURI_INTERNALS__" in window;
}

function sanitizeProjectFileName(title: string) {
  const sanitized = title
    .trim()
    .replace(/[<>:"/\\|?*\u0000-\u001f]/g, "-")
    .replace(/\s+/g, " ")
    .slice(0, 80)
    .trim();

  return sanitized || "untitled-lesson";
}

function withProjectPackageExtension(path: string) {
  return path.toLowerCase().endsWith(".llpkg") ? path : `${path}.llpkg`;
}

function saveProjectSummary(result: SaveProjectPackageResponse) {
  const parts = [`Saved ${result.bundledAssetCount} assets`];

  if (result.missingAssetCount > 0) {
    parts.push(`${result.missingAssetCount} missing`);
  }

  if (result.externalAssetCount > 0) {
    parts.push(`${result.externalAssetCount} external`);
  }

  if (result.warnings.length > 0) {
    parts.push(`${result.warnings.length} warnings`);
  }

  return parts.join(" / ");
}

function loadOpenedProject(result: OpenProjectPackageResponse) {
  packageResourceUrls.value = resourceUrlMapFromOpenedProject(result);
  currentPackagePath.value = result.packagePath;
  currentSourceFilePath.value = result.sourceFilePath;
  currentWorkingDirectory.value = result.workingDirectory;
  source.value = result.source;
  activeSlideId.value = "slide-1";
  projectSaveStatus.value = {
    kind: "success",
    message: openProjectSummary(result),
    warnings: result.warnings,
  };

  nextTick(() => {
    previewStageRef.value?.scrollTo({ top: 0, behavior: "auto" });
  });
}

function resourceUrlMapFromOpenedProject(result: OpenProjectPackageResponse) {
  const map: Record<string, string> = {};

  for (const asset of result.assets) {
    if (asset.status !== "bundled" || !asset.filePath) {
      continue;
    }

    const fileUrl = convertFileSrc(asset.filePath);
    const { base, suffix } = splitResourceSuffix(asset.source);

    map[asset.source] = `${fileUrl}${suffix}`;
    map[base] ??= fileUrl;
  }

  return map;
}

function resolvePreviewResourceUrl(src: string) {
  const exact = packageResourceUrls.value[src];

  if (exact) {
    return exact;
  }

  const { base, suffix } = splitResourceSuffix(src);
  const baseUrl = packageResourceUrls.value[base];

  return baseUrl ? `${baseUrl}${suffix}` : src;
}

function splitResourceSuffix(src: string) {
  const markerIndex = src.search(/[?#]/);

  if (markerIndex === -1) {
    return { base: src, suffix: "" };
  }

  return {
    base: src.slice(0, markerIndex),
    suffix: src.slice(markerIndex),
  };
}

function openProjectSummary(result: OpenProjectPackageResponse) {
  const parts = [`Opened ${result.bundledAssetCount} assets`];

  if (result.missingAssetCount > 0) {
    parts.push(`${result.missingAssetCount} missing`);
  }

  if (result.externalAssetCount > 0) {
    parts.push(`${result.externalAssetCount} external`);
  }

  if (result.warnings.length > 0) {
    parts.push(`${result.warnings.length} warnings`);
  }

  return parts.join(" / ");
}
</script>

<template>
  <WindowFrame ref="windowFrameRef" :title="course.title">
  <main
    v-if="!isPresentationMode"
    class="course-shell paper-bg"
    :class="{ 'navigator-collapsed': !isNavigatorOpen }"
    aria-label="Liveleaf markdown workspace"
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
        <strong>{{ course.title }}</strong>
      </div>

      <div class="lesson-actions">
        <button
          class="save-project-button present-button"
          type="button"
          title="Start presentation"
          @click="enterPresentation"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M8 4H4v4" />
            <path d="M16 4h4v4" />
            <path d="M20 16v4h-4" />
            <path d="M4 16v4h4" />
            <path d="M9 9h6v6H9z" />
          </svg>
          <span>Present</span>
        </button>

        <button
          class="save-project-button"
          type="button"
          :disabled="isProjectBusy"
          :aria-busy="isOpeningProject"
          title="Open project package"
          @click="handleOpenProject"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 7h6l2 2h8v10H4z" />
            <path d="M12 14h6" />
            <path d="m15 11 3 3-3 3" />
          </svg>
          <span>{{ isOpeningProject ? "Opening" : "Open Project" }}</span>
        </button>

        <button
          class="save-project-button"
          type="button"
          :disabled="isProjectBusy"
          :aria-busy="isSavingProject"
          title="Save project package"
          @click="handleSaveProject"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M5 4h10l4 4v12H5z" />
            <path d="M8 4v6h8" />
            <path d="M8 17h8" />
          </svg>
          <span>{{ isSavingProject ? "Saving" : "Save Project" }}</span>
        </button>

        <p
          v-if="projectSaveStatus.kind !== 'idle'"
          class="save-project-status"
          :class="`save-project-status--${projectSaveStatus.kind}`"
          :title="projectSaveStatus.warnings?.join('\n')"
        >
          {{ projectSaveStatus.message }}
        </p>

        <div class="lesson-stats" aria-label="Document stats">
          <span v-if="currentPackagePath" title="Loaded package">.llpkg</span>
          <span>{{ course.codeCells.length }} code</span>
          <span>{{ source.length }} chars</span>
        </div>
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
          <strong>.ll.md</strong>
        </div>

        <CodeEditor
          v-model="source"
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
          @click="handlePreviewClick"
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

            <Teleport
              v-for="cell in visibleRunCells(slide)"
              :key="cell.id"
              :to="runtimeOutputTarget(cell.id)"
              defer
            >
              <section
                class="runtime-output"
                :class="`runtime-output--${runStateFor(cell.id)?.status ?? 'pending'}`"
                :aria-label="`${cell.id} runtime output`"
              >
                <header class="runtime-output__header">
                  <div>
                    <span>Output</span>
                    <strong>{{ cell.id }}</strong>
                  </div>
                  <span class="runtime-output__status">
                    {{ statusLabel(runStateFor(cell.id)?.status) }}
                  </span>
                </header>

                <p
                  v-if="runStateFor(cell.id)?.status === 'pending'"
                  class="runtime-output__message"
                >
                  Running locally...
                </p>

                <p
                  v-else-if="runStateFor(cell.id)?.message"
                  class="runtime-output__message runtime-output__message--error"
                >
                  {{ runStateFor(cell.id)?.message }}
                </p>

                <template v-if="runStateFor(cell.id)?.result">
                  <pre
                    v-if="runStateFor(cell.id)?.result?.stdout"
                    class="runtime-output__stream"
                  ><code>{{ runStateFor(cell.id)?.result?.stdout }}</code></pre>

                  <pre
                    v-if="runStateFor(cell.id)?.result?.stderr"
                    class="runtime-output__stream runtime-output__stream--stderr"
                  ><code>{{ runStateFor(cell.id)?.result?.stderr }}</code></pre>

                  <p
                    v-if="runStateFor(cell.id)?.result?.error?.message"
                    class="runtime-output__message"
                    :class="{
                      'runtime-output__message--error':
                        runStateFor(cell.id)?.result?.status !== 'success',
                    }"
                  >
                    {{ runStateFor(cell.id)?.result?.error?.message }}
                  </p>

                  <span class="runtime-output__duration">
                    {{ formatDuration(runStateFor(cell.id)?.result) }}
                  </span>
                  <p v-if="runStateFor(cell.id)?.visualizationError" class="runtime-output__message runtime-output__message--error">{{ runStateFor(cell.id)?.visualizationError }}</p>
                  <TracePlayer v-if="runStateFor(cell.id)?.trace" :trace="runStateFor(cell.id)!.trace!" :source="runStateFor(cell.id)?.sourceSnapshot ?? ''" :document-line="runStateFor(cell.id)?.documentLine" :run-status="runStateFor(cell.id)?.status ?? 'error'" :stale="runStateFor(cell.id)?.stale" />
                </template>
              </section>
            </Teleport>
          </article>
        </div>
      </section>
    </div>
  </main>

  <section
    v-else
    ref="presentationRootRef"
    class="presentation-shell paper-bg"
    tabindex="-1"
    aria-label="全屏演示"
    @click="handlePresentationClick"
  >
    <header class="presentation-toolbar" aria-label="演示控制栏">
      <div class="presentation-title">
        <span>Liveleaf</span>
        <strong>{{ activeSlide?.title ?? course.title }}</strong>
      </div>

      <div class="presentation-actions">
        <button
          class="presentation-control"
          type="button"
          :disabled="isFirstSlide"
          title="Previous slide"
          aria-label="上一页"
          @click="goToPreviousSlide"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="m15 6-6 6 6 6" />
          </svg>
        </button>

        <span class="presentation-counter" aria-label="当前页码">
          {{ presentationSlideLabel }}
        </span>

        <button
          class="presentation-control"
          type="button"
          :disabled="isLastSlide"
          title="Next slide"
          aria-label="下一页"
          @click="goToNextSlide"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="m9 6 6 6-6 6" />
          </svg>
        </button>

        <button
          class="presentation-control presentation-control--exit"
          type="button"
          title="Exit presentation"
          aria-label="退出演示"
          @click="exitPresentation"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M18 6 6 18" />
            <path d="m6 6 12 12" />
          </svg>
        </button>
      </div>
    </header>

    <div
      ref="presentationScrollRef"
      class="presentation-scroll"
      aria-label="当前幻灯片内容"
    >
      <article
        v-if="activeSlide"
        :key="activeSlide.id"
        class="presentation-slide"
        :aria-labelledby="`presentation-slide-title-${activeSlide.id}`"
      >
        <div class="slide-meta presentation-slide-meta">
          <span>Slide {{ activeSlide.index + 1 }}</span>
          <strong v-if="activeSlide.codeCells.length">
            {{ activeSlide.codeCells.length }} code
          </strong>
        </div>

        <div
          :id="`presentation-slide-title-${activeSlide.id}`"
          class="markdown-preview presentation-markdown"
          v-html="activeSlide.html"
        />

        <Teleport
          v-for="cell in visibleRunCells(activeSlide)"
          :key="cell.id"
          :to="runtimeOutputTarget(cell.id)"
          defer
        >
          <section
            class="runtime-output"
            :class="`runtime-output--${runStateFor(cell.id)?.status ?? 'pending'}`"
            :aria-label="`${cell.id} runtime output`"
          >
            <header class="runtime-output__header">
              <div>
                <span>Output</span>
                <strong>{{ cell.id }}</strong>
              </div>
              <span class="runtime-output__status">
                {{ statusLabel(runStateFor(cell.id)?.status) }}
              </span>
            </header>

            <p
              v-if="runStateFor(cell.id)?.status === 'pending'"
              class="runtime-output__message"
            >
              Running locally...
            </p>

            <p
              v-else-if="runStateFor(cell.id)?.message"
              class="runtime-output__message runtime-output__message--error"
            >
              {{ runStateFor(cell.id)?.message }}
            </p>

            <template v-if="runStateFor(cell.id)?.result">
              <pre
                v-if="runStateFor(cell.id)?.result?.stdout"
                class="runtime-output__stream"
              ><code>{{ runStateFor(cell.id)?.result?.stdout }}</code></pre>

              <pre
                v-if="runStateFor(cell.id)?.result?.stderr"
                class="runtime-output__stream runtime-output__stream--stderr"
              ><code>{{ runStateFor(cell.id)?.result?.stderr }}</code></pre>

              <p
                v-if="runStateFor(cell.id)?.result?.error?.message"
                class="runtime-output__message"
                :class="{
                  'runtime-output__message--error':
                    runStateFor(cell.id)?.result?.status !== 'success',
                }"
              >
                {{ runStateFor(cell.id)?.result?.error?.message }}
              </p>

              <span class="runtime-output__duration">
                {{ formatDuration(runStateFor(cell.id)?.result) }}
              </span>
              <p v-if="runStateFor(cell.id)?.visualizationError" class="runtime-output__message runtime-output__message--error">{{ runStateFor(cell.id)?.visualizationError }}</p>
              <TracePlayer v-if="runStateFor(cell.id)?.trace" :trace="runStateFor(cell.id)!.trace!" :source="runStateFor(cell.id)?.sourceSnapshot ?? ''" :document-line="runStateFor(cell.id)?.documentLine" :run-status="runStateFor(cell.id)?.status ?? 'error'" :stale="runStateFor(cell.id)?.stale" />
            </template>
          </section>
        </Teleport>
      </article>
    </div>
  </section>
  </WindowFrame>
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
  --scrollbar-thumb: rgba(51, 68, 84, 0.34);
  --scrollbar-thumb-hover: rgba(51, 68, 84, 0.52);
  color: var(--ink-dark);
  background: transparent;
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
  scrollbar-color: var(--scrollbar-thumb) transparent;
  scrollbar-width: thin;
}

*::-webkit-scrollbar {
  width: 10px;
  height: 10px;
}

*::-webkit-scrollbar-track,
*::-webkit-scrollbar-track-piece,
*::-webkit-scrollbar-corner {
  background: transparent;
}

*::-webkit-scrollbar-button {
  display: none;
  width: 0;
  height: 0;
}

*::-webkit-scrollbar-thumb {
  border: 2px solid transparent;
  border-radius: 999px;
  background: var(--scrollbar-thumb);
  background-clip: padding-box;
}

*::-webkit-scrollbar-thumb:hover {
  background: var(--scrollbar-thumb-hover);
  background-clip: padding-box;
}

body {
  min-width: 320px;
  height: 100dvh;
  margin: 0;
  overflow: hidden;
  background: transparent;
}

#app {
  height: 100%;
  background: transparent;
}

button,
textarea {
  font: inherit;
}
</style>

<style scoped>
.course-shell {
  height: 100%;
  min-height: 0;
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
.lesson-actions,
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
.save-project-button:focus-visible,
.presentation-control:focus-visible,
.slide-thumbnail:focus-visible,
.slide-dot:focus-visible {
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

.lesson-actions {
  min-width: 0;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 8px;
}

.save-project-button {
  min-height: 44px;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 7px 12px;
  border: 4px solid var(--ink);
  border-radius: 10px;
  background: var(--surface);
  color: var(--ink-dark);
  box-shadow: 6px 6px 0 var(--shadow);
  cursor: pointer;
  font-weight: 900;
  white-space: nowrap;
  transition:
    transform 180ms cubic-bezier(0.2, 0.8, 0.2, 1),
    box-shadow 180ms cubic-bezier(0.2, 0.8, 0.2, 1),
    opacity 180ms cubic-bezier(0.2, 0.8, 0.2, 1);
}

.save-project-button svg {
  width: 22px;
  height: 22px;
  flex: 0 0 auto;
  fill: none;
  stroke: currentColor;
  stroke-width: 3;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.save-project-button:hover:not(:disabled) {
  transform: translate(-2px, -2px);
  box-shadow: 8px 8px 0 var(--shadow);
}

.save-project-button:active:not(:disabled) {
  transform: translate(2px, 2px);
  box-shadow: 3px 3px 0 var(--shadow);
}

.save-project-button:disabled {
  cursor: wait;
  opacity: 0.72;
}

.present-button {
  background: var(--yellow);
}

.save-project-status {
  max-width: min(320px, 100%);
  margin: 0;
  padding: 5px 10px;
  overflow: hidden;
  border: 3px solid var(--ink);
  border-radius: 8px;
  background: var(--paper-soft);
  color: var(--ink-dark);
  font-size: 0.82rem;
  font-weight: 900;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.save-project-status--success {
  border-color: var(--green);
}

.save-project-status--error {
  border-color: var(--red);
  color: #9b2721;
}

.save-project-status--opening,
.save-project-status--saving {
  border-color: var(--blue);
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
  overflow-x: hidden;
  overflow-y: auto;
  scrollbar-gutter: stable;
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
  width: 48px;
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
  max-width: 100%;
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

.markdown-preview :deep(img) {
  display: block;
  max-width: 100%;
  height: auto;
  margin: 20px 0;
  border: 4px solid var(--ink);
  border-radius: 12px;
  background: var(--paper-soft);
  box-shadow: 8px 8px 0 rgba(47, 65, 82, 0.16);
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
  max-width: 100%;
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

.markdown-preview :deep(.sl-code-cell__identity) {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.markdown-preview :deep(.sl-code-cell__identity strong) {
  overflow-wrap: anywhere;
}

.markdown-preview :deep(.sl-code-cell__language-icon) {
  display: block;
  flex-shrink: 0;
}

.markdown-preview :deep(.sl-code-cell__actions) {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  margin-left: auto;
}

.markdown-preview :deep(.sl-code-cell__timeout) {
  white-space: nowrap;
  font-weight: 600;
}

.markdown-preview :deep(.sl-code-cell__run) {
  min-height: 30px;
  padding: 3px 12px;
  border: 3px solid var(--ink);
  border-radius: 999px;
  background: var(--yellow);
  color: var(--ink-dark);
  cursor: pointer;
  font-family: inherit;
  font-size: 0.82rem;
  font-weight: 900;
}

.markdown-preview :deep(.sl-code-cell__diagnostic) {
  margin: 8px 16px;
  padding: 10px;
  border-left: 4px solid var(--yellow);
  color: #fffdf5;
  font-size: .85rem;
}

.markdown-preview :deep(.sl-code-cell__run:hover) {
  transform: translate(-1px, -1px);
  box-shadow: 3px 3px 0 rgba(47, 65, 82, 0.28);
}

.markdown-preview :deep(.sl-code-cell > pre) {
  max-width: 100%;
  margin: 0;
  padding: 20px;
  overflow: auto;
  background: transparent;
  color: var(--surface);
  line-height: 1.68;
}

.markdown-preview :deep(.sl-code-cell__output-slot:not(:empty)) {
  padding: 0 16px 16px;
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
  max-width: 100%;
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

.runtime-output {
  padding: 14px 16px;
  border: 4px solid var(--ink);
  border-radius: 10px;
  background: var(--paper-soft);
  color: var(--ink-dark);
  box-shadow: 8px 8px 0 rgba(47, 65, 82, 0.14);
}

.runtime-output--success {
  border-color: var(--green);
}

.runtime-output--error,
.runtime-output--timeout {
  border-color: var(--red);
}

.runtime-output__header {
  min-height: 34px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}

.runtime-output__header div {
  min-width: 0;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.runtime-output__header span {
  color: var(--green);
  font-weight: 900;
}

.runtime-output__header strong {
  min-width: 0;
  color: var(--ink-dark);
  font-family: "JetBrains Mono", "SFMono-Regular", Consolas, monospace;
  font-size: 0.9rem;
  overflow-wrap: anywhere;
}

.runtime-output__status,
.runtime-output__duration {
  width: max-content;
  padding: 3px 9px;
  border: 3px solid var(--ink);
  border-radius: 999px;
  background: var(--yellow);
  color: var(--ink-dark);
  font-size: 0.78rem;
  font-weight: 900;
}

.runtime-output__message {
  margin: 0;
  color: var(--ink-dark);
  font-weight: 900;
}

.runtime-output__message--error {
  color: #9b2721;
}

.runtime-output__stream {
  margin: 10px 0 0;
  padding: 14px;
  overflow: auto;
  border: 3px solid var(--ink);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink-dark);
  font-family: "JetBrains Mono", "SFMono-Regular", Consolas, monospace;
  font-size: 0.9rem;
  line-height: 1.55;
  white-space: pre-wrap;
}

.markdown-preview :deep(.sl-code-cell .runtime-output__stream) {
  padding: 14px;
  background: var(--surface);
  color: var(--ink-dark);
  line-height: 1.55;
}

.runtime-output__stream--stderr {
  border-color: var(--red);
  background: #fff3ec;
  color: #8f241f;
}

.markdown-preview :deep(.sl-code-cell .runtime-output__stream--stderr) {
  background: #fff3ec;
  color: #8f241f;
}

.runtime-output__stream code {
  color: inherit;
}

.markdown-preview :deep(.sl-code-cell .runtime-output__stream code) {
  color: inherit;
}

.runtime-output__duration {
  display: inline-grid;
  margin-top: 10px;
  place-items: center;
}

.presentation-shell {
  width: 100%;
  height: 100%;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  overflow: hidden;
  color: var(--ink-dark);
  outline: none;
}

.presentation-shell:fullscreen {
  width: 100vw;
  height: 100vh;
}

.presentation-toolbar {
  min-height: 76px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  padding: 12px 24px;
  border-bottom: 5px solid var(--ink);
  background: rgba(251, 247, 234, 0.9);
}

.presentation-title {
  min-width: 0;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  color: var(--green);
  font-weight: 900;
}

.presentation-title strong {
  min-width: 0;
  color: var(--ink-dark);
  font-family: Georgia, "Times New Roman", serif;
  font-size: 1.35rem;
  overflow-wrap: anywhere;
}

.presentation-actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 10px;
}

.presentation-control {
  width: 46px;
  height: 46px;
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
    box-shadow 180ms cubic-bezier(0.2, 0.8, 0.2, 1),
    opacity 180ms cubic-bezier(0.2, 0.8, 0.2, 1);
}

.presentation-control svg {
  width: 25px;
  height: 25px;
  fill: none;
  stroke: currentColor;
  stroke-width: 3.4;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.presentation-control:hover:not(:disabled) {
  transform: translate(-2px, -2px);
  box-shadow: 8px 8px 0 var(--shadow);
}

.presentation-control:active:not(:disabled) {
  transform: translate(2px, 2px);
  box-shadow: 3px 3px 0 var(--shadow);
}

.presentation-control:disabled {
  cursor: default;
  opacity: 0.46;
}

.presentation-control--exit {
  background: var(--red);
  color: var(--surface);
}

.presentation-counter {
  min-width: 76px;
  min-height: 40px;
  display: grid;
  place-items: center;
  padding: 4px 12px;
  border: 4px solid var(--ink);
  border-radius: 999px;
  background: var(--yellow);
  color: var(--ink-dark);
  font-weight: 900;
  white-space: nowrap;
}

.presentation-scroll {
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
  padding: clamp(18px, 3vw, 34px) clamp(16px, 4vw, 48px)
    clamp(28px, 5vw, 60px);
}

.presentation-slide {
  width: min(1180px, 100%);
  min-height: calc(100vh - 160px);
  margin: 0 auto;
  padding: clamp(24px, 4vw, 52px);
  border: 4px solid var(--ink);
  border-radius: 8px;
  background: var(--surface);
  box-shadow: 14px 14px 0 rgba(47, 65, 82, 0.2);
}

.presentation-slide-meta {
  margin-bottom: 30px;
}

.presentation-markdown {
  font-size: 1.08rem;
  line-height: 1.62;
}

.presentation-shell .markdown-preview :deep(h1) {
  font-size: 4rem;
}

.presentation-shell .markdown-preview :deep(h2) {
  font-size: 2.1rem;
}

.presentation-shell .markdown-preview :deep(.sl-code-cell > pre),
.presentation-shell .markdown-preview :deep(:not(.sl-code-cell) > pre) {
  max-height: 38vh;
}

.presentation-shell .runtime-output__stream {
  max-height: 30vh;
}

.presentation-shell .markdown-preview :deep(.sl-media video) {
  max-height: 58vh;
  object-fit: contain;
}

.presentation-shell .markdown-preview :deep(img) {
  max-height: 58vh;
  object-fit: contain;
}

.presentation-shell .markdown-preview :deep(.sl-media audio) {
  max-height: 72px;
}

.navigator-collapsed .top-rule {
  grid-template-columns: 72px minmax(0, 1fr) auto;
}

.navigator-collapsed .workspace-layout {
  grid-template-columns: 72px minmax(300px, 0.92fr) minmax(360px, 1.12fr);
}

.navigator-collapsed .slide-navigator {
  padding-right: 8px;
}

@media (max-width: 1120px) {
  .top-rule {
    grid-template-columns: 170px minmax(0, 1fr);
    gap: 12px;
  }

  .lesson-actions {
    grid-column: 1 / -1;
    justify-content: flex-start;
  }

  .workspace-layout,
  .navigator-collapsed .workspace-layout {
    grid-template-columns: 150px minmax(0, 1fr);
    grid-template-rows: minmax(360px, 0.88fr) minmax(420px, 1.12fr);
    overflow-y: auto;
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

  .lesson-actions {
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
    width: 100%;
    grid-template-columns: repeat(auto-fill, 42px);
    grid-auto-flow: row;
    justify-content: start;
  }

  .navigator-collapsed .slide-navigator {
    padding-right: 0;
  }

  .preview-stage {
    overflow: visible;
    padding-right: 0;
  }

  .presentation-toolbar {
    align-items: flex-start;
    flex-direction: column;
    gap: 12px;
    padding: 12px 16px;
  }

  .presentation-actions {
    width: 100%;
    justify-content: space-between;
  }

  .presentation-scroll {
    padding: 16px 16px 32px;
  }

  .presentation-slide {
    min-height: calc(100vh - 176px);
    padding: 24px 18px;
    border-width: 3px;
    box-shadow: 10px 10px 0 rgba(47, 65, 82, 0.2);
  }

  .presentation-shell .markdown-preview :deep(h1) {
    font-size: 3rem;
  }

  .presentation-shell .markdown-preview :deep(.sl-columns) {
    flex-wrap: wrap;
  }

  .presentation-shell .markdown-preview :deep(.sl-column) {
    flex-basis: 100%;
    padding: 14px 0 0;
  }

  .presentation-shell .markdown-preview :deep(.sl-column + .sl-column) {
    margin-top: 14px;
    border-top: 3px solid var(--ink);
    border-left: 0;
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

  .save-project-status {
    max-width: 100%;
    white-space: normal;
  }

  .markdown-preview :deep(h1) {
    font-size: clamp(2.3rem, 15vw, 3.6rem);
    text-shadow: 3px 3px 0 var(--ink);
  }

  .presentation-title strong {
    font-size: 1.1rem;
  }

  .presentation-control {
    width: 42px;
    height: 42px;
    border-width: 3px;
  }

  .presentation-counter {
    min-width: 64px;
    min-height: 38px;
    border-width: 3px;
    font-size: 0.9rem;
  }

  .presentation-markdown {
    font-size: 1rem;
  }

  .presentation-shell .markdown-preview :deep(h1) {
    font-size: 2.55rem;
  }

  .presentation-shell .markdown-preview :deep(h2) {
    font-size: 1.65rem;
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
