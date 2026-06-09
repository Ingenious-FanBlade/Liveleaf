# Technical Architecture

## 1. Overview

Stream Learn is a local-first desktop application built with Vue and Tauri.

The application has five main layers:

```text
Application UI
Document Model
Markdown Parser
Runtime System
Platform Services
```

## 2. Layer Responsibilities

## 2.1 Application UI

The UI is responsible for user workflows:

- Course file opening and saving.
- Markdown editing.
- Preview rendering.
- Presentation mode.
- Code block execution controls.
- Runtime output display.
- Settings.

Suggested main screens:

- Editor.
- Preview.
- Presentation.
- Resource manager.
- Settings.

## 2.2 Document Model

The document model is the internal representation of a course file.

It should represent:

- Markdown blocks.
- Slides.
- Code cells.
- Layout containers.
- Media blocks.
- Exercise blocks.
- Runtime outputs.

The document model should be independent from Vue components. Vue renders the
document model, but it should not be the only place where parsing logic lives.

## 2.3 Markdown Parser

The parser converts `.md` or `.sl.md` source text into the document model.

Recommended direction:

- Use the unified ecosystem if practical.
- Use `remark-parse` for Markdown.
- Use `remark-gfm` for GitHub Flavored Markdown.
- Use `remark-directive` or a similar container syntax parser for extensions.
- Convert parsed Markdown AST into a Stream Learn document AST.

The parser must preserve enough source information to map rendered blocks back
to their source ranges when possible.

## 2.4 Runtime System

The runtime system executes embedded code blocks.

Runtime responsibilities:

- Identify executable code blocks.
- Choose a runtime adapter by language.
- Send code to the adapter.
- Capture stdout, stderr, rich output, and errors.
- Enforce timeout and cancellation.
- Return a normalized run result.

The first runtime adapter is Python.

The runtime layer should be language-agnostic. Python is the first adapter, not
the only possible runtime.

## 2.5 Platform Services

Tauri provides platform capabilities:

- File system access.
- Native dialogs.
- Process execution.
- Local configuration.
- Optional database or sidecar state.

Tauri commands should be small and explicit. The frontend should not need to know
low-level process or file details.

## 3. Suggested Frontend Modules

```text
src/
  app/
    routes/
    shell/
  features/
    editor/
    preview/
    presentation/
    runtime/
    files/
    settings/
  markdown/
    parseCourse.ts
    documentModel.ts
    directives.ts
  runtime/
    RuntimeAdapter.ts
    PythonRuntimeClient.ts
    runResult.ts
  components/
    markdown/
    layout/
    media/
    code/
```

## 4. Suggested Tauri Modules

```text
src-tauri/src/
  lib.rs
  commands/
    files.rs
    python_runtime.rs
    settings.rs
  runtime/
    python.rs
    process.rs
  model/
    run_result.rs
```

The actual Rust module layout can evolve, but runtime execution should not be
mixed into generic file commands.

## 5. Course File Model

Recommended source file extensions:

- `.sl.md` for Stream Learn course files.
- `.md` for standard Markdown files opened in compatibility mode.

Recommended folder layout:

```text
course/
  lesson-01.sl.md
  assets/
    intro.mp4
    graph.png
  tests/
    two_sum_test.py
  .streamlearn/
    outputs.json
    student-edits.json
    settings.json
```

## 6. Runtime Result Model

All runtimes should return the same result shape.

```ts
export interface RunResult {
  id: string;
  language: string;
  status: "success" | "error" | "timeout" | "cancelled";
  stdout: string;
  stderr: string;
  outputs: RichOutput[];
  startedAt: string;
  endedAt: string;
  durationMs: number;
  error?: RuntimeError;
}

export interface RichOutput {
  kind: "text" | "html" | "markdown" | "image" | "json";
  mime?: string;
  value: string;
}

export interface RuntimeError {
  name: string;
  message: string;
  traceback?: string;
}
```

## 7. Persistence

MVP persistence can use sidecar JSON files before introducing a database.

Suggested sidecar files:

- `.streamlearn/outputs.json`
- `.streamlearn/student-edits.json`
- `.streamlearn/session.json`

This keeps the course source file clean and avoids writing outputs into the
Markdown file unless the user explicitly exports them.

Future versions may use SQLite for:

- Course library indexing.
- Full-text search.
- Recent files.
- Runtime history.
- Student progress.

## 8. Security Model

Python code execution must be treated as local code execution.

MVP rules:

- Do not run code automatically when opening a document.
- Require user action to run code.
- Make it clear that code runs on the local machine.
- Use a timeout.
- Prefer a temporary working directory for execution.
- Limit environment variables passed to child processes.

Future rules:

- Trust prompts for external course files.
- Per-course runtime permissions.
- Optional container or sandbox mode.
- Network access control where possible.

## 9. Export Targets

Future export targets:

- Static HTML.
- PDF.
- Slide deck.
- Markdown with outputs.
- Student handout version.
- Teacher version with solutions.

MVP can defer export except for saving the source file.

