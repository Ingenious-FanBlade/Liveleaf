# Product Requirements

## 1. Product Positioning

Stream Learn is a desktop teaching and learning tool for computer science
courses.

It is designed to replace PowerPoint in programming-related teaching scenarios
where static slides are not enough. Teachers should be able to write course
materials in Markdown, present them like slides, and run embedded code directly
inside the material.

The product sits between PowerPoint and Jupyter Notebook:

- It is lighter and more version-control-friendly than PowerPoint.
- It is more presentation-oriented and layout-aware than Jupyter Notebook.
- It keeps the source format readable, portable, and mostly standard Markdown.

## 2. Core Goal

Allow teachers to create courseware with Markdown and teach from the same file
they give to students.

The same document should support:

- Reading.
- Presenting.
- Running code.
- Viewing output.
- Embedding media.
- Completing exercises.
- Exporting or sharing.

## 3. Target Users

### Teachers

Teachers need to prepare computer science course materials quickly, demonstrate
code live, and distribute readable learning materials to students.

Common needs:

- Write course content without complex slide editing.
- Keep course files in Git.
- Run code examples during class.
- Avoid switching among PowerPoint, IDE, terminal, browser, and video player.
- Reuse and update materials across semesters.

### Students

Students need a readable course document that can also run examples and support
practice.

Common needs:

- Read the same materials used in class.
- Re-run examples after class.
- Modify code snippets.
- Complete exercises in context.
- Keep local notes or outputs without damaging the original course file.

## 4. Primary Scenarios

### 4.1 Teacher Authoring

A teacher writes a lesson in Markdown:

- Normal Markdown for text.
- Code fences for examples.
- Extra syntax for columns, videos, hints, and exercises.
- Slide separators for presentation pages.

### 4.2 Classroom Presentation

A teacher opens the file and enters presentation mode:

- Pages are shown like slides.
- Code blocks can be run in place.
- Output appears below or beside the code.
- Font size and theme can be adjusted.
- The teacher can move forward and backward quickly.

### 4.3 Student Self-Study

A student opens the same file:

- Reads the material in document mode.
- Runs Python code examples.
- Edits code in runnable cells.
- Checks exercise hints or solutions when allowed.

### 4.4 Course Distribution

The teacher shares a course folder:

- A `.sl.md` or `.md` file.
- An `assets/` directory for images, videos, and other files.
- Optional exercise tests and runtime metadata.

## 5. Functional Requirements

## 5.1 Markdown Authoring

The application must support standard Markdown as the base format.

Required Markdown features:

- Headings.
- Paragraphs.
- Lists.
- Blockquotes.
- Tables.
- Links.
- Images.
- Inline code.
- Fenced code blocks.

Recommended compatibility target:

- GitHub Flavored Markdown where reasonable.

## 5.2 Slide Presentation

The application must be able to split a Markdown file into presentation pages.

MVP behavior:

- Use `---` as a slide separator when it appears between block-level content.
- Each slide is rendered as one presentation page.
- Presentation mode supports next and previous navigation.
- Current slide index is tracked.

Future behavior:

- Support vertical slide groups.
- Support presenter notes.
- Support fragments and step-by-step reveals.

## 5.3 Layout Extensions

The application should support simple layout extensions while keeping the source
file readable.

MVP extensions:

- Two-column and three-column layout.
- Column width hints.
- Callouts.
- Hints and solutions.
- Video and audio embedding.

The extension syntax should not break normal Markdown rendering too badly. If a
file is opened in another Markdown viewer, the content should remain mostly
understandable as plain text.

## 5.4 Runnable Python Code Blocks

The first runtime target is Python.

MVP behavior:

- A Python code block marked with `run` can be executed.
- Standard output is captured.
- Standard error is captured.
- Exceptions are shown with readable tracebacks.
- Execution can time out.
- Output is displayed near the code block.

Example:

````markdown
```python run
print("Hello, Stream Learn")
```
````

## 5.5 Exercise Blocks

The application should support exercise-style code blocks.

MVP behavior:

- Exercise code can be edited by students.
- Exercise blocks can include an identifier.
- The application can save student edits separately from the original source.

Future behavior:

- Hidden tests.
- Public tests.
- Grading result display.
- Exported student submissions.

## 5.6 Media Embedding

The application should support media inside teaching materials.

MVP media types:

- Local image files.
- Local video files.
- Local audio files.
- Remote media URLs when the platform permits them.

Video blocks should support controls and optional start time.

## 5.7 Editing and Preview

MVP interface:

- Left side: Markdown editor.
- Right side: rendered preview.
- Toolbar: open, save, run current block, presentation mode, export.

The editor does not need to become a full IDE in the first phase.

## 5.8 Local Files

The product should be local-first.

MVP expectations:

- Course files are stored on disk.
- Assets are referenced by relative paths.
- Runtime outputs and student edits can be stored in a local sidecar directory.

Suggested course folder:

```text
lesson-01/
  lesson.sl.md
  assets/
    diagram.png
    demo.mp4
  .streamlearn/
    outputs.json
    student-edits.json
```

## 6. Non-Functional Requirements

## 6.1 Portability

Course source files should be plain text and Git-friendly.

## 6.2 Safety

Python execution is powerful and risky. The runtime must make execution scope
clear to the user.

MVP safety expectations:

- Show that code is executed locally.
- Use execution timeouts.
- Use an isolated working directory where practical.
- Do not silently grant broad file-system access.

## 6.3 Performance

MVP expectations:

- Open normal course files quickly.
- Render slide pages without noticeable delay.
- Run small Python examples interactively.

## 6.4 Offline Use

The application should work offline for local course files and local Python code
execution.

## 7. MVP Scope

The MVP should include:

1. Open and save Markdown-based course files.
2. Render Markdown preview.
3. Split documents into slides with `---`.
4. Enter presentation mode.
5. Render columns, callouts, videos, hints, and solutions.
6. Run Python code blocks marked with `run`.
7. Display stdout, stderr, and exceptions.
8. Store code outputs locally.

## 8. Explicit Non-Goals For MVP

The MVP should not include:

- Cloud sync.
- Multi-user collaboration.
- LMS integration.
- AI-generated lessons.
- Online judge infrastructure.
- Complex freeform slide canvas editing.
- Full Jupyter `.ipynb` compatibility.
- Support for every programming language.

