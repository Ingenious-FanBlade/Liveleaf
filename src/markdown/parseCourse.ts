import MarkdownIt from "markdown-it";
import markdownItContainer from "markdown-it-container";
import type { CodeCell, CourseDocument, CourseSlide } from "./documentModel";

interface ParsedParams {
  flags: Set<string>;
  attrs: Record<string, string>;
}

interface CodeInfo {
  language: string;
  flags: Set<string>;
  attrs: Record<string, string>;
}

interface SlideSource {
  index: number;
  source: string;
}

interface ContainerToken {
  info: string;
  nesting: number;
  type: string;
}

const directiveNames = [
  "note",
  "tip",
  "warning",
  "danger",
  "question",
  "hint",
  "solution",
];

const calloutLabels: Record<string, string> = {
  note: "Note",
  tip: "Tip",
  warning: "Warning",
  danger: "Danger",
  question: "Question",
};

const revealLabels: Record<string, string> = {
  hint: "Hint",
  solution: "Solution",
};

export function parseCourse(source: string): CourseDocument {
  const slideSources = splitSlides(source);
  const slides = slideSources.map((slideSource) => renderSlide(slideSource));
  const codeCells = slides.flatMap((slide) => slide.codeCells);

  return {
    title: findDocumentTitle(slides) ?? "Untitled Lesson",
    source,
    slides,
    codeCells,
  };
}

function renderSlide(slideSource: SlideSource): CourseSlide {
  const codeCells: CodeCell[] = [];
  const markdown = createMarkdownRenderer(slideSource.index, codeCells);
  const html = markdown.render(slideSource.source);

  return {
    id: `slide-${slideSource.index + 1}`,
    index: slideSource.index,
    title: findSlideTitle(slideSource.source) ?? `Slide ${slideSource.index + 1}`,
    source: slideSource.source,
    html,
    codeCells,
  };
}

function createMarkdownRenderer(slideIndex: number, codeCells: CodeCell[]) {
  const markdown = new MarkdownIt({
    breaks: false,
    html: false,
    linkify: true,
    typographer: false,
  });

  for (const name of directiveNames) {
    markdown.use(markdownItContainer, name, {
      validate: (params: string) => {
        return new RegExp(`^${name}(?:\\s|$)`).test(params.trim());
      },
      render: (tokens: ContainerToken[], index: number) => {
        return renderContainerDirective(markdown, tokens[index]);
      },
    });
  }

  markdown.use(columnsDirectivePlugin);
  markdown.use(mediaDirectivePlugin);

  const defaultFence =
    markdown.renderer.rules.fence ??
    ((tokens, index, options, _env, self) => self.renderToken(tokens, index, options));

  markdown.renderer.rules.fence = (tokens, index, options, env, self) => {
    const token = tokens[index];
    const info = parseCodeInfo(token.info);
    const kind = info.flags.has("exercise")
      ? "exercise"
      : info.flags.has("run")
        ? "run"
        : null;

    if (!kind) {
      return defaultFence(tokens, index, options, env, self);
    }

    const id = info.attrs.id ?? `slide-${slideIndex + 1}-cell-${codeCells.length + 1}`;
    const timeout = parseInteger(info.attrs.timeout);
    const language = info.language || "text";
    const sourceRange = token.map
      ? {
          startLine: token.map[0] + 1,
          endLine: token.map[1],
        }
      : undefined;

    codeCells.push({
      id,
      language,
      kind,
      code: token.content,
      timeout,
      session: info.attrs.session,
      sourceRange,
      slideIndex,
    });

    return renderCodeCell(markdown, {
      code: token.content,
      id,
      kind,
      language,
      session: info.attrs.session,
      timeout,
    });
  };

  return markdown;
}

function splitSlides(source: string): SlideSource[] {
  const normalizedSource = source.replace(/\r\n?/g, "\n");
  const lines = normalizedSource.split("\n");
  const slides: SlideSource[] = [];
  let currentLines: string[] = [];
  let inFence = false;
  let fenceMarker = "";
  let inFrontmatter = lines[0]?.trim() === "---";

  lines.forEach((line, lineIndex) => {
    if (inFrontmatter) {
      currentLines.push(line);

      if (lineIndex > 0 && isSlideSeparator(line)) {
        inFrontmatter = false;
      }

      return;
    }

    const fence = line.match(/^\s{0,3}(`{3,}|~{3,})/);
    if (fence) {
      const marker = fence[1][0];

      if (!inFence) {
        inFence = true;
        fenceMarker = marker;
      } else if (marker === fenceMarker) {
        inFence = false;
        fenceMarker = "";
      }
    }

    if (!inFence && isSlideSeparator(line) && hasMeaningfulContent(currentLines)) {
      slides.push({
        index: slides.length,
        source: currentLines.join("\n").trim(),
      });
      currentLines = [];
      return;
    }

    currentLines.push(line);
  });

  slides.push({
    index: slides.length,
    source: currentLines.join("\n").trim(),
  });

  return slides.length > 0 ? slides : [{ index: 0, source: "" }];
}

function renderContainerDirective(
  markdown: MarkdownIt,
  token: ContainerToken,
) {
  const { name, params } = parseContainerTokenInfo(token);

  if (token.nesting === -1) {
    if (name === "hint" || name === "solution") {
      return "</details>\n";
    }

    if (name in calloutLabels) {
      return "</aside>\n";
    }

    return name === "column" ? "</section>\n" : "</div>\n";
  }

  if (name === "columns") {
    return '<div class="sl-columns">\n';
  }

  if (name === "column") {
    const width = safeCssWidth(params.attrs.width);
    const style = width ? ` style="--column-width: ${markdown.utils.escapeHtml(width)}"` : "";
    return `<section class="sl-column"${style}>\n`;
  }

  if (name === "hint" || name === "solution") {
    const label = revealLabels[name];
    const isOpen = params.flags.has("open") || !params.flags.has("collapsed");
    const openAttr = name === "hint" ? "" : isOpen ? " open" : "";
    return `<details class="sl-reveal sl-reveal--${name}"${openAttr}><summary>${label}</summary>\n`;
  }

  if (name in calloutLabels) {
    const label = calloutLabels[name];
    return `<aside class="sl-callout sl-callout--${name}"><div class="sl-callout__label">${label}</div>\n`;
  }

  return `<div class="sl-directive sl-directive--${markdown.utils.escapeHtml(name)}">\n`;
}

function parseContainerTokenInfo(token: ContainerToken) {
  if (token.info.trim()) {
    return parseDirectiveInfo(token.info);
  }

  return {
    name: token.type.match(/^container_(.+)_(?:open|close)$/)?.[1] ?? "",
    params: parseParams(""),
  };
}

function columnsDirectivePlugin(markdown: MarkdownIt) {
  markdown.block.ruler.before(
    "paragraph",
    "stream_learn_columns",
    (state: any, startLine: number, endLine: number, silent: boolean) => {
      const openingLine = getLineText(state, startLine).trim();

      if (!isColumnsStart(openingLine)) {
        return false;
      }

      if (silent) {
        return true;
      }

      const columns: Array<{ source: string; width?: string }> = [];
      let line = startLine + 1;

      while (line < endLine) {
        const currentLine = getLineText(state, line).trim();

        if (currentLine.length === 0) {
          line += 1;
          continue;
        }

        if (isContainerClose(currentLine)) {
          if (columns.length === 0) {
            return false;
          }

          const token = state.push("stream_learn_columns", "", 0);
          token.block = true;
          token.map = [startLine, line + 1];
          token.meta = { columns };
          state.line = line + 1;

          return true;
        }

        if (!isColumnStart(currentLine)) {
          return false;
        }

        const { params } = parseDirectiveInfo(currentLine.replace(/^:::\s*/, ""));
        const columnEndLine = findColumnEndLine(state, line + 1, endLine);

        if (columnEndLine >= endLine) {
          return false;
        }

        columns.push({
          source: state.getLines(line + 1, columnEndLine, 0, false).trim(),
          width: safeCssWidth(params.attrs.width),
        });

        line = columnEndLine + 1;
      }

      return false;
    },
  );

  markdown.renderer.rules.stream_learn_columns = (tokens: any[], index: number) => {
    const meta = tokens[index].meta as {
      columns: Array<{ source: string; width?: string }>;
    };
    const renderedColumns = meta.columns
      .map((column) => {
        const style = column.width
          ? ` style="--column-width: ${markdown.utils.escapeHtml(column.width)}"`
          : "";
        return `<section class="sl-column"${style}>\n${markdown.render(column.source)}</section>`;
      })
      .join("\n");

    return `<div class="sl-columns">\n${renderedColumns}\n</div>\n`;
  };
}

function mediaDirectivePlugin(markdown: MarkdownIt) {
  markdown.block.ruler.before(
    "paragraph",
    "stream_learn_media",
    (state: any, startLine: number, _endLine: number, silent: boolean) => {
      const position = state.bMarks[startLine] + state.tShift[startLine];
      const max = state.eMarks[startLine];
      const line = state.src.slice(position, max);
      const match = line.match(
        /^::(video|audio)\[([^\]]*)\]\(([^)]*)\)(?:\{([^}]*)\})?\s*$/,
      );

      if (!match) {
        return false;
      }

      if (silent) {
        return true;
      }

      const token = state.push("stream_learn_media", "", 0);
      token.block = true;
      token.map = [startLine, startLine + 1];
      token.meta = {
        kind: match[1],
        label: match[2],
        src: match[3],
        params: parseParams(match[4] ?? ""),
      };
      state.line = startLine + 1;

      return true;
    },
  );

  markdown.renderer.rules.stream_learn_media = (tokens: any[], index: number) => {
    const meta = tokens[index].meta as {
      kind: "audio" | "video";
      label: string;
      src: string;
      params: ParsedParams;
    };
    const label = markdown.utils.escapeHtml(meta.label);
    const src = markdown.utils.escapeHtml(resourceUrlWithStart(meta.src, meta.params.attrs.start));
    const width = safeCssWidth(meta.params.attrs.width);
    const style = width ? ` style="--media-width: ${markdown.utils.escapeHtml(width)}"` : "";
    const controls = meta.params.flags.has("controls") ? " controls" : "";

    if (meta.kind === "audio") {
      return `<figure class="sl-media sl-media--audio"${style}><audio src="${src}"${controls}></audio>${label ? `<figcaption>${label}</figcaption>` : ""}</figure>\n`;
    }

    const poster = meta.params.attrs.poster
      ? ` poster="${markdown.utils.escapeHtml(meta.params.attrs.poster)}"`
      : "";

    return `<figure class="sl-media sl-media--video"${style}><video src="${src}"${controls}${poster}></video>${label ? `<figcaption>${label}</figcaption>` : ""}</figure>\n`;
  };
}

function findColumnEndLine(state: any, startLine: number, endLine: number) {
  let directiveDepth = 0;
  let fenceMarker = "";
  let fenceLength = 0;

  for (let line = startLine; line < endLine; line += 1) {
    const lineText = getLineText(state, line);
    const trimmedLine = lineText.trim();
    const fenceMatch = lineText.match(/^\s{0,3}(`{3,}|~{3,})/);

    if (fenceMatch) {
      const marker = fenceMatch[1][0];
      const markerLength = fenceMatch[1].length;

      if (!fenceMarker) {
        fenceMarker = marker;
        fenceLength = markerLength;
      } else if (marker === fenceMarker && markerLength >= fenceLength) {
        fenceMarker = "";
        fenceLength = 0;
      }

      continue;
    }

    if (fenceMarker) {
      continue;
    }

    if (isDirectiveStart(trimmedLine) && !isColumnStart(trimmedLine)) {
      directiveDepth += 1;
      continue;
    }

    if (isContainerClose(trimmedLine)) {
      if (directiveDepth > 0) {
        directiveDepth -= 1;
        continue;
      }

      return line;
    }
  }

  return endLine;
}

function renderCodeCell(
  markdown: MarkdownIt,
  cell: {
    code: string;
    id: string;
    kind: "run" | "exercise";
    language: string;
    session?: string;
    timeout?: number;
  },
) {
  const escapedCode = markdown.utils.escapeHtml(cell.code);
  const escapedId = markdown.utils.escapeHtml(cell.id);
  const escapedLanguage = markdown.utils.escapeHtml(cell.language);
  const metaItems = [
    cell.kind,
    cell.language,
    cell.session ? `session:${cell.session}` : null,
    cell.timeout ? `${cell.timeout}ms` : null,
  ].filter(Boolean);
  const metaHtml = metaItems
    .map((item) => `<span>${markdown.utils.escapeHtml(String(item))}</span>`)
    .join("");

  return `<section class="sl-code-cell sl-code-cell--${cell.kind}" data-code-cell-id="${escapedId}"><div class="sl-code-cell__header"><strong>${escapedId}</strong><div>${metaHtml}</div></div><pre><code class="language-${escapedLanguage}">${escapedCode}</code></pre></section>\n`;
}

function parseCodeInfo(info: string): CodeInfo {
  const trimmedInfo = info.trim();
  const [language = "", ...rest] = trimmedInfo ? trimmedInfo.split(/\s+/) : [];
  const params = parseParams(rest.join(" "));

  return {
    language,
    flags: params.flags,
    attrs: params.attrs,
  };
}

function parseDirectiveInfo(info: string) {
  const [name = "", ...rest] = info.trim().split(/\s+/);

  return {
    name,
    params: parseParams(rest.join(" ")),
  };
}

function parseParams(params: string): ParsedParams {
  const flags = new Set<string>();
  const attrs: Record<string, string> = {};
  const paramPattern = /([^\s"'=]+)(?:=(?:"([^"]*)"|'([^']*)'|([^\s]+)))?/g;
  let match: RegExpExecArray | null;

  while ((match = paramPattern.exec(params)) !== null) {
    const key = match[1];
    const value = match[2] ?? match[3] ?? match[4];

    if (value === undefined) {
      flags.add(key);
    } else {
      attrs[key] = value;
    }
  }

  return { flags, attrs };
}

function parseInteger(value: string | undefined) {
  if (!value) {
    return undefined;
  }

  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function isSlideSeparator(line: string) {
  return /^\s{0,3}---\s*$/.test(line);
}

function getLineText(state: any, line: number) {
  const start = state.bMarks[line] + state.tShift[line];
  const end = state.eMarks[line];

  return state.src.slice(start, end);
}

function isColumnsStart(line: string) {
  return /^:::\s+columns(?:\s|$)/.test(line);
}

function isColumnStart(line: string) {
  return /^:::\s+column(?:\s|$)/.test(line);
}

function isDirectiveStart(line: string) {
  return /^:::\s+\S+/.test(line);
}

function isContainerClose(line: string) {
  return line === ":::";
}

function hasMeaningfulContent(lines: string[]) {
  return lines.some((line) => line.trim().length > 0);
}

function findDocumentTitle(slides: CourseSlide[]) {
  return slides.map((slide) => slide.title).find((title) => !/^Slide \d+$/.test(title));
}

function findSlideTitle(source: string) {
  return source.match(/^#\s+(.+)$/m)?.[1].trim();
}

function safeCssWidth(value: string | undefined) {
  if (!value) {
    return undefined;
  }

  return /^\d+(?:\.\d+)?(?:%|px|rem|em)$/.test(value) ? value : undefined;
}

function resourceUrlWithStart(src: string, start: string | undefined) {
  const trimmedSrc = src.trim();

  if (!start || !/^\d+(?:\.\d+)?$/.test(start)) {
    return trimmedSrc;
  }

  return `${trimmedSrc}#t=${start}`;
}
