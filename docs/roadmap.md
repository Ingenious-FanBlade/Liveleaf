# Roadmap

## Phase 0: Design Baseline

Status: current phase.

Goals:

- Define product direction.
- Define Markdown extension syntax.
- Define Python runtime architecture.
- Keep MVP scope small enough to implement.

Deliverables:

- Product requirements.
- Architecture notes.
- Markdown syntax design.
- Python runtime design.

## Phase 1: Markdown Course Viewer

Goals:

- Open a Markdown file.
- Render Markdown preview.
- Split slides by `---`.
- Enter presentation mode.

Key work:

- Markdown parser.
- Document model.
- Preview renderer.
- Presentation navigation.
- Basic theme.

## Phase 2: Markdown Extensions

Goals:

- Support layout and teaching-specific blocks.

Key work:

- Columns.
- Callouts.
- Hints.
- Solutions.
- Video blocks.
- Audio blocks.

## Phase 3: Python Runtime MVP

Goals:

- Run Python code blocks from Markdown.
- Display stdout, stderr, exceptions, and timeout results.

Key work:

- Runtime TypeScript interfaces.
- Tauri `python_run` command.
- Process-per-run Python execution.
- Executable code block component.
- Output display component.

## Phase 4: Fullscreen Presentation And Project Package

Goals:

- Make Stream Learn usable for classroom presentation and easy distribution.

Key work:

- Fullscreen presentation mode.
- Keyboard navigation and presentation controls.
- Presentation-safe media rendering for images, videos, and audio.
- Save project command.
- Stream Learn project package format.
- Bundle Markdown source, project metadata, images, videos, audio, and other
  referenced assets into one distributable file.
- Open and validate packaged projects.
- Resource path rewriting inside packaged projects.

## Phase 5: Markdown Highlighting And Functional Symbols

Goals:

- Make Markdown authoring clearer by highlighting code, text, and Stream Learn
  functional syntax.

Key work:

- Syntax highlighting for fenced code blocks.
- Inline code highlighting.
- Markdown text highlight settings.
- Highlight Stream Learn functional symbols, including slide separators,
  runnable code markers, exercise markers, directives, media blocks, hints, and
  solutions.
- Theme-aware highlight colors for editor, preview, and presentation mode.
- Highlight settings saved with the project.

## Phase 6: Authoring Experience

Goals:

- Make the tool useful for teachers preparing lessons.

Key work:

- Split editor and preview.
- Run current code block.
- Course resource path handling.
- Basic settings.
- Project metadata editing.
- Unsaved-change handling.

## Phase 7: Student Exercise Flow

Goals:

- Support basic student practice inside course files.

Key work:

- Editable exercise blocks.
- Sidecar student edits.
- Exercise run button.
- Optional public tests.
- Result display.

## Phase 8: Better Runtime Sessions

Goals:

- Move closer to notebook-style Python execution.

Key work:

- Session-scoped Python process.
- Shared state across blocks.
- Restart session.
- Interrupt execution.
- Session status UI.

## Phase 9: Export Beyond Project Packages

Goals:

- Support additional sharing formats after the native package format is stable.

Key work:

- Static HTML export.
- Teacher and student export modes.
- Optional PDF export.
- Compatibility checks for packaged projects.
