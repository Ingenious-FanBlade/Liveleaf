# Liveleaf Documentation

Liveleaf is a Markdown-first teaching tool for computer science courses. It
combines lightweight slide authoring, notebook-style executable code blocks, and
student-friendly self-study materials.

This documentation set is the first project design baseline.

## Documents

- [Product Requirements](./product-requirements.md)
- [Technical Architecture](./technical-architecture.md)
- [Markdown Syntax Design](./markdown-syntax.md)
- [CLI 与 AI 自动化接口](./cli.md)
- [Python Runtime Design](./python-runtime-design.md)
- [Python Hook 可视化功能设计](./python-visualization-hook-design.md)
- [可运行示例：数组与二叉树先序遍历](./examples/python-visualization.ll.md)
- [Python Hook 有向图扩展](./python-graph-visualization.md)
- [Python Hook Dynamic Table](./python-dynamic-table.md)
- [可运行示例：有向图、Dijkstra 最短路径与动态表格](./examples/dijkstra.ll.md)
- [Roadmap](./roadmap.md)

## Current Direction

The first development phase focuses on:

1. Markdown-compatible course files.
2. Slide-style presentation from Markdown.
3. Lightweight layout extensions such as columns and media blocks.
4. Embedded runnable Python code blocks.
5. A local-first desktop experience built with Vue and Tauri.

## Working Name

The project name is `liveleaf`.

The product meaning is:

> A lightweight stream of teaching content where explanations, code, output,
> media, and exercises can live in one Markdown-based learning document.

