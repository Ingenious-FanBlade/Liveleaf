# Markdown Syntax Design

## 1. Design Principles

Stream Learn source files should be Markdown-first.

The syntax design follows these principles:

1. Standard Markdown remains valid.
2. Extensions are readable as plain text.
3. Extensions should be block-oriented where possible.
4. Course files should work well with Git diff.
5. The same source should support reading, presenting, and running code.

## 2. File Extension

Recommended extension:

```text
.sl.md
```

Reason:

- It is still clearly Markdown.
- It signals Stream Learn extensions.
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
print("Hello, Stream Learn")
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

