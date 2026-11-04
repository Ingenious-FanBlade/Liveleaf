# Markdown Syntax Design

## 1. Design Principles

Liveleaf source files should be Markdown-first.

The syntax design follows these principles:

1. Standard Markdown remains valid.
2. Extensions are readable as plain text.
3. Extensions should be block-oriented where possible.
4. Course files should work well with Git diff.
5. The same source should support reading, presenting, and running code.

## 2. File Extension

Recommended extension:

```text
.ll.md
```

Reason:

- It is still clearly Markdown.
- It signals Liveleaf extensions.
- Editors can still treat it as Markdown.

Normal `.md` files should still open, but extension features may be limited.

## 3. Slides

Use a horizontal rule as the default slide separator.

```markdown
# Lesson 1

This is the first slide.

---

# Variables

This is the second slide.
```

MVP behavior:

- `---` splits slides in presentation mode.
- In document mode, it can still render as a visual separator.

Future options:

```markdown
---
layout: title
---

# Lesson 1
```

Frontmatter support can be added later.

## 4. Runnable Code Blocks

Runnable code uses a fenced code block with a `run` marker.

````markdown
```python run
print("Hello, Liveleaf")
```
````

With options:

````markdown
```python run id="intro-print" timeout=5000
print("Hello")
```
````

Supported MVP attributes:

- `id`: stable block identifier.
- `timeout`: timeout in milliseconds.
- `session`: optional session name.

If no `id` is provided, the application may derive one from the file path and
block index. Authors should use explicit IDs for stable output tracking.

## 5. Python Exercise Blocks

Exercise code blocks use the `exercise` marker.

````markdown
```python exercise id="sum-list"
def sum_list(values):
    # TODO
    pass
```
````

MVP behavior:

- Render as editable code.
- Save student edits separately from source.
- Run like normal Python code when requested.

Future attributes:

````markdown
```python exercise id="sum-list" tests="./tests/sum_list_test.py"
def sum_list(values):
    pass
```
````

## 6. Columns

Use container directives for columns.

```markdown
::: columns
::: column width=45%
## Concept

A variable stores a value.
:::

::: column width=55%
```python run
x = 42
print(x)
```
:::
:::
```

MVP behavior:

- Render child blocks horizontally when there is enough width.
- Stack columns vertically on narrow screens.
- `width` is treated as a hint, not an exact contract.

## 7. Callouts

Callouts use named containers.

```markdown
::: note
This is an important note.
:::

::: warning
This code mutates the original list.
:::

::: tip
Use meaningful variable names.
:::
```

Suggested callout types:

- `note`
- `tip`
- `warning`
- `danger`
- `question`

## 8. Hints And Solutions

Hints:

```markdown
::: hint
Think about what state needs to be remembered.
:::
```

Solutions:

```markdown
::: solution collapsed
Use a dictionary to store previously seen values.
:::
```

MVP behavior:

- `hint` is collapsible by default.
- `solution collapsed` is hidden until expanded.
- Presentation mode may allow teacher settings to reveal or hide solutions.

## 9. Video

Use a leaf directive for videos.

```markdown
::video[Sorting demo](./assets/sort.mp4){controls start=12}
```

Attributes:

- `controls`: show video controls.
- `start`: start time in seconds.
- `width`: optional width hint.
- `poster`: optional poster image.

Remote example:

```markdown
::video[Demo](https://example.com/demo.mp4){controls}
```

## 10. Audio

```markdown
::audio[Narration](./assets/intro.mp3){controls}
```

## 11. Images

Standard Markdown image syntax should be enough for most cases.

```markdown
![Graph](./assets/graph.png)
```

Optional future enhanced image syntax:

```markdown
::image[Graph](./assets/graph.png){width=60% caption="Runtime model"}
```

## 12. Presenter Notes

Future syntax:

```markdown
::: notes
Mention that Python variables are references to objects.
:::
```

MVP can parse but hide notes from normal student view if implemented.

## 13. Metadata

Future frontmatter:

```markdown
---
title: Python Basics
author: Teacher Name
theme: default
runtime:
  python:
    version: "3.11"
---
```

MVP does not require frontmatter, but the parser should not make it impossible
to add later.

## 14. Frontend Document Model And Rust Handoff

The current MVP parser lives in the frontend so the editor can provide live
preview while the user types. It converts Markdown source into a lightweight
document model:

```ts
interface SourceRange {
  startLine: number;
  endLine: number;
}

interface CodeCell {
  id: string;
  language: string;
  kind: "run" | "exercise";
  code: string;
  timeout?: number;
  session?: string;
  sourceRange?: SourceRange;
  slideIndex: number;
}

interface CourseSlide {
  id: string;
  index: number;
  title: string;
  source: string;
  html: string;
  codeCells: CodeCell[];
}

interface CourseDocument {
  title: string;
  source: string;
  slides: CourseSlide[];
  codeCells: CodeCell[];
}
```

This model is an application view of the source file, not a new file format.
The Markdown source remains the canonical data. The model can be rebuilt at any
time from the source.

Frontend responsibilities in the MVP:

- Parse source text for live preview.
- Split slides by `---`.
- Render Markdown and Liveleaf directives into preview HTML.
- Identify runnable and exercise code cells for UI controls.
- Track source line ranges when the parser can provide them.

Future Rust responsibilities:

- Parse the same source text into a compatible `CourseDocument` shape.
- Re-identify executable code cells before running code.
- Validate cell IDs, attributes, source ranges, and runtime options.
- Provide authoritative parsing for file open, save checks, execution, export,
  indexing, and security-sensitive operations.
- Avoid trusting frontend-only metadata when executing code.

Recommended Tauri handoff:

```text
Frontend editor source
  -> frontend parseCourse(source) for live preview
  -> Tauri command parse_course(source) for authoritative Rust model
  -> Tauri command run_code_cell(source, cell_id) for execution
```

The backend should receive the full Markdown source and a stable cell ID when
running code. It should parse the source again, locate the matching `CodeCell`,
then execute that cell. This prevents the frontend from accidentally or
maliciously sending code that does not match the visible document.

Schema compatibility rules:

- Keep field names JSON-friendly and shared between TypeScript and Rust.
- Rust structs should derive `Serialize` and `Deserialize` and mirror the
  frontend model.
- Add a `schemaVersion` field before the model becomes part of saved sidecar
  state or export artifacts.
- Treat `html` as a frontend preview cache in the MVP. The long-term Rust model
  may return semantic blocks instead of HTML, especially for export targets.
- Keep `sourceRange` line-based and 1-based for UI display. Parser internals may
  use 0-based positions, but the serialized model should stay user-facing.

Expected Rust mirror:

```rust
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseDocument {
    pub title: String,
    pub source: String,
    pub slides: Vec<CourseSlide>,
    pub code_cells: Vec<CodeCell>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseSlide {
    pub id: String,
    pub index: usize,
    pub title: String,
    pub source: String,
    pub html: Option<String>,
    pub code_cells: Vec<CodeCell>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeCell {
    pub id: String,
    pub language: String,
    pub kind: CodeCellKind,
    pub code: String,
    pub timeout: Option<u64>,
    pub session: Option<String>,
    pub source_range: Option<SourceRange>,
    pub slide_index: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRange {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CodeCellKind {
    Run,
    Exercise,
}
```

Implementation notes for parity:

- Use the same slide separator rule in both parsers.
- Keep directive names and code fence attributes identical across frontend and
  backend.
- Add parser fixture tests that run the same `.ll.md` examples through both
  parsers and compare slide count, titles, code cell IDs, kinds, languages, and
  runtime attributes.
- Do not require perfect HTML parity between frontend and backend. Semantic
  parity matters more than identical rendered markup.
