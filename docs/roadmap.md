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

## Phase 4: Authoring Experience

Goals:

- Make the tool useful for teachers preparing lessons.

Key work:

- Split editor and preview.
- Save files.
- Run current code block.
- Course resource path handling.
- Basic settings.

## Phase 5: Student Exercise Flow

Goals:

- Support basic student practice inside course files.

Key work:

- Editable exercise blocks.
- Sidecar student edits.
- Exercise run button.
- Optional public tests.
- Result display.

## Phase 6: Better Runtime Sessions

Goals:

- Move closer to notebook-style Python execution.

Key work:

- Session-scoped Python process.
- Shared state across blocks.
- Restart session.
- Interrupt execution.
- Session status UI.

## Phase 7: Export And Distribution

Goals:

- Help teachers share course materials.

Key work:

- Static HTML export.
- Teacher and student export modes.
- Optional PDF export.
- Course package format.

