use std::collections::{HashMap, HashSet};

use crate::runtime::visualization::{self, VisualizationConfig, VisualizationDiagnostic};
use ::markdown::mdast::Node;
use ::markdown::{to_mdast, ParseOptions};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CourseDocument {
    pub title: String,
    pub source: String,
    pub slides: Vec<CourseSlide>,
    pub code_cells: Vec<CodeCell>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CourseSlide {
    pub id: String,
    pub index: usize,
    pub title: String,
    pub source: String,
    pub html: Option<String>,
    pub code_cells: Vec<CodeCell>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visualization: Option<VisualizationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visualization_diagnostics: Option<Vec<VisualizationDiagnostic>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceRange {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CodeCellKind {
    Run,
    Exercise,
}

#[derive(Debug, Clone)]
struct SlideSource {
    index: usize,
    source: String,
    start_line: usize,
}

#[derive(Debug, Default)]
pub(crate) struct ParsedParams {
    pub flags: HashSet<String>,
    pub attrs: HashMap<String, String>,
    pub duplicates: HashSet<String>,
    pub unterminated_quote: bool,
}

pub fn parse_course(source: &str) -> Result<CourseDocument, String> {
    let normalized_source = normalize_line_endings(source);
    let slide_sources = split_slides(&normalized_source);
    let mut slides = Vec::with_capacity(slide_sources.len());

    for slide_source in slide_sources {
        slides.push(parse_slide(slide_source)?);
    }

    let code_cells = slides
        .iter()
        .flat_map(|slide| slide.code_cells.iter().cloned())
        .collect::<Vec<_>>();
    let title = find_document_title(&slides).unwrap_or_else(|| "Untitled Lesson".to_string());

    Ok(CourseDocument {
        title,
        source: normalized_source,
        slides,
        code_cells,
    })
}

fn parse_slide(slide_source: SlideSource) -> Result<CourseSlide, String> {
    let options = ParseOptions::gfm();
    let ast = to_mdast(&slide_source.source, &options)
        .map_err(|error| format!("failed to parse slide {}: {error}", slide_source.index + 1))?;

    let mut code_cells = Vec::new();
    collect_code_cells(
        &ast,
        slide_source.index,
        slide_source.start_line,
        &mut code_cells,
    );

    let title =
        find_first_h1_title(&ast).unwrap_or_else(|| format!("Slide {}", slide_source.index + 1));

    Ok(CourseSlide {
        id: format!("slide-{}", slide_source.index + 1),
        index: slide_source.index,
        title,
        source: slide_source.source,
        html: None,
        code_cells,
    })
}

fn collect_code_cells(
    node: &Node,
    slide_index: usize,
    line_offset: usize,
    code_cells: &mut Vec<CodeCell>,
) {
    if let Node::Code(code) = node {
        let params = parse_params(code.meta.as_deref().unwrap_or(""));
        let kind = if params.flags.contains("exercise") {
            Some(CodeCellKind::Exercise)
        } else if params.flags.contains("run") {
            Some(CodeCellKind::Run)
        } else {
            None
        };

        if let Some(kind) = kind {
            let id = params.attrs.get("id").cloned().unwrap_or_else(|| {
                format!("slide-{}-cell-{}", slide_index + 1, code_cells.len() + 1)
            });
            let language = code.lang.clone().unwrap_or_else(|| "text".to_string());
            let timeout = params
                .attrs
                .get("timeout")
                .and_then(|value| value.parse::<u64>().ok());
            let source_range = code.position.as_ref().map(|position| SourceRange {
                start_line: line_offset + position.start.line.saturating_sub(1),
                end_line: line_offset + position.end.line.saturating_sub(1),
            });
            let (visualization, diagnostics) = visualization::parse_config(
                &language,
                &params.attrs,
                &params.duplicates,
                &params.flags,
            );

            code_cells.push(CodeCell {
                id,
                language,
                kind,
                code: code.value.clone(),
                timeout,
                session: params.attrs.get("session").cloned(),
                source_range,
                slide_index,
                visualization,
                visualization_diagnostics: if diagnostics.is_empty() {
                    None
                } else {
                    Some(diagnostics)
                },
            });
        }
    }

    if let Some(children) = node.children() {
        for child in children {
            collect_code_cells(child, slide_index, line_offset, code_cells);
        }
    }
}

fn find_first_h1_title(node: &Node) -> Option<String> {
    if let Node::Heading(heading) = node {
        if heading.depth == 1 {
            let title = collect_plain_text(&heading.children).trim().to_string();

            if !title.is_empty() {
                return Some(title);
            }
        }
    }

    node.children()?
        .iter()
        .find_map(|child| find_first_h1_title(child))
}

fn collect_plain_text(nodes: &[Node]) -> String {
    let mut text = String::new();

    for node in nodes {
        match node {
            Node::Text(value) => text.push_str(&value.value),
            Node::InlineCode(value) => text.push_str(&value.value),
            _ => {
                if let Some(children) = node.children() {
                    text.push_str(&collect_plain_text(children));
                }
            }
        }
    }

    text
}

fn split_slides(source: &str) -> Vec<SlideSource> {
    let lines = source.split('\n').collect::<Vec<_>>();
    let mut slides = Vec::new();
    let mut current_lines = Vec::new();
    let mut in_fence = false;
    let mut fence_marker = None;
    let mut fence_length = 0;
    let mut in_frontmatter = lines.first().is_some_and(|line| line.trim() == "---");

    for (line_index, line) in lines.iter().enumerate() {
        let line_number = line_index + 1;

        if in_frontmatter {
            current_lines.push((line_number, (*line).to_string()));

            if line_index > 0 && is_slide_separator(line) {
                in_frontmatter = false;
            }

            continue;
        }

        if let Some((marker, length, closing)) = fence_marker_for_line(line) {
            if !in_fence {
                in_fence = true;
                fence_marker = Some(marker);
                fence_length = length;
            } else if fence_marker == Some(marker) && length >= fence_length && closing {
                in_fence = false;
                fence_marker = None;
            }
        }

        if !in_fence && is_slide_separator(line) && has_meaningful_content(&current_lines) {
            slides.push(build_slide(slides.len(), &current_lines));
            current_lines.clear();
            continue;
        }

        current_lines.push((line_number, (*line).to_string()));
    }

    slides.push(build_slide(slides.len(), &current_lines));

    if slides.is_empty() {
        vec![SlideSource {
            index: 0,
            source: String::new(),
            start_line: 1,
        }]
    } else {
        slides
    }
}

fn build_slide(index: usize, lines: &[(usize, String)]) -> SlideSource {
    let first = lines
        .iter()
        .position(|(_, line)| !line.trim().is_empty())
        .unwrap_or(0);
    let end = lines
        .iter()
        .rposition(|(_, line)| !line.trim().is_empty())
        .map(|i| i + 1)
        .unwrap_or(first);
    let joined = lines[first..end]
        .iter()
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    SlideSource {
        index,
        source: joined,
        start_line: lines.get(first).map(|(line, _)| *line).unwrap_or(1),
    }
}

fn normalize_line_endings(source: &str) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}

/// Source lines which must not be interpreted as teaching/media directives.
pub(crate) fn code_lines(node: &Node) -> HashSet<usize> {
    let mut lines = HashSet::new();
    if matches!(node, Node::Code(_) | Node::InlineCode(_)) {
        if let Some(position) = node.position() {
            lines.extend(position.start.line..=position.end.line);
        }
    }
    if let Some(children) = node.children() {
        for child in children {
            lines.extend(code_lines(child));
        }
    }
    lines
}

pub(crate) fn parse_params(params: &str) -> ParsedParams {
    let mut parsed = ParsedParams::default();
    let mut chars = params.chars().peekable();

    loop {
        skip_whitespace(&mut chars);

        if chars.peek().is_none() {
            break;
        }

        let key = read_until(&mut chars, |ch| ch.is_whitespace() || ch == '=');

        if key.is_empty() {
            chars.next();
            continue;
        }

        if chars.peek() == Some(&'=') {
            chars.next();
            let (mut value, complete) = read_param_value(&mut chars);
            parsed.unterminated_quote |= !complete;
            if !complete
                && [
                    "visualize",
                    "watch",
                    "pointers",
                    "scope",
                    "value",
                    "children",
                    "node_states",
                    "edge_states",
                    "tables",
                    "distances",
                    "visited",
                    "predecessors",
                ]
                .contains(&key.as_str())
            {
                value.insert(0, '"');
            }
            if parsed.attrs.contains_key(&key) || parsed.flags.contains(&key) {
                parsed.duplicates.insert(key.clone());
            }
            parsed.attrs.insert(key, value);
        } else {
            if parsed.attrs.contains_key(&key) || parsed.flags.contains(&key) {
                parsed.duplicates.insert(key.clone());
            }
            parsed.flags.insert(key);
        }
    }

    parsed
}

fn read_param_value<I>(chars: &mut std::iter::Peekable<I>) -> (String, bool)
where
    I: Iterator<Item = char>,
{
    match chars.peek().copied() {
        Some('"') | Some('\'') => {
            let quote = chars.next().expect("peeked quote should exist");
            let value = read_until(chars, |ch| ch == quote);

            let complete = chars.peek() == Some(&quote);
            if complete {
                chars.next();
            }

            (value, complete)
        }
        Some(_) => (read_until(chars, |ch| ch.is_whitespace()), true),
        None => (String::new(), true),
    }
}

fn read_until<I, F>(chars: &mut std::iter::Peekable<I>, should_stop: F) -> String
where
    I: Iterator<Item = char>,
    F: Fn(char) -> bool,
{
    let mut value = String::new();

    while let Some(ch) = chars.peek().copied() {
        if should_stop(ch) {
            break;
        }

        value.push(ch);
        chars.next();
    }

    value
}

fn skip_whitespace<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while chars.peek().is_some_and(|ch| ch.is_whitespace()) {
        chars.next();
    }
}

fn is_slide_separator(line: &str) -> bool {
    let leading_whitespace = line.len() - line.trim_start().len();

    leading_whitespace <= 3 && line.trim() == "---"
}

fn fence_marker_for_line(line: &str) -> Option<(char, usize, bool)> {
    let leading_whitespace = line.len() - line.trim_start().len();

    if leading_whitespace > 3 {
        return None;
    }

    let trimmed = line.trim_start();

    let marker = if trimmed.starts_with("```") {
        '`'
    } else if trimmed.starts_with("~~~") {
        '~'
    } else {
        return None;
    };
    let length = trimmed.chars().take_while(|ch| *ch == marker).count();
    Some((marker, length, trimmed[length..].trim().is_empty()))
}

fn has_meaningful_content(lines: &[(usize, String)]) -> bool {
    lines.iter().any(|(_, line)| !line.trim().is_empty())
}

fn find_document_title(slides: &[CourseSlide]) -> Option<String> {
    slides
        .iter()
        .map(|slide| slide.title.as_str())
        .find(|title| !is_fallback_title(title))
        .map(ToString::to_string)
}

fn is_fallback_title(title: &str) -> bool {
    title
        .strip_prefix("Slide ")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_visualization_fixtures_match_frontend() {
        let fixtures: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../../tests/fixtures/visualization.json")).unwrap();
        for (i, fixture) in fixtures.iter().enumerate() {
            let document = parse_course(fixture["source"].as_str().unwrap()).unwrap();
            if let Some(slides) = fixture["slides"].as_u64() {
                assert_eq!(document.slides.len(), slides as usize);
            }
            if fixture["static"] == true {
                assert!(document.code_cells.is_empty());
                continue;
            }
            let cell = &document.code_cells[0];
            if let Some(config) = fixture.get("config") {
                assert_eq!(
                    serde_json::to_value(&cell.visualization).unwrap(),
                    *config,
                    "fixture {i}"
                );
            }
            if let Some(errors) = fixture.get("errors") {
                let attributes: Vec<_> = cell
                    .visualization_diagnostics
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|d| d.attribute.as_deref().unwrap())
                    .collect();
                assert_eq!(serde_json::json!(attributes), *errors, "fixture {i}");
                assert!(cell.visualization.is_none());
            }
            if let Some(line) = fixture["line"].as_u64() {
                assert_eq!(
                    cell.source_range.as_ref().unwrap().start_line,
                    line as usize,
                    "fixture {i}"
                );
            }
        }
    }

    #[test]
    fn parses_gfm_documents() {
        let document =
            parse_course("# GFM\n\n| A | B |\n| - | - |\n| 1 | 2 |\n\n- [x] done\n\n~~old~~\n")
                .expect("GFM markdown should parse");

        assert_eq!(document.title, "GFM");
        assert_eq!(document.slides.len(), 1);
        assert_eq!(document.code_cells.len(), 0);
    }

    #[test]
    fn splits_slides_without_splitting_inside_fences() {
        let document = parse_course(
            "# One\n\n```python run id=\"keep\"\nprint(\"---\")\n```\n\n---\n\n# Two\n",
        )
        .expect("course should parse");

        assert_eq!(document.slides.len(), 2);
        assert_eq!(document.slides[0].title, "One");
        assert_eq!(document.slides[1].title, "Two");
        assert_eq!(document.code_cells.len(), 1);
        assert_eq!(document.code_cells[0].id, "keep");
        assert_eq!(document.code_cells[0].slide_index, 0);
    }

    #[test]
    fn keeps_frontmatter_on_the_first_slide() {
        let document = parse_course("---\ntitle: Example\n---\n\n# Intro\n\n---\n\n# Next\n")
            .expect("course with frontmatter should parse");

        assert_eq!(document.slides.len(), 2);
        assert!(document.slides[0]
            .source
            .starts_with("---\ntitle: Example\n---"));
        assert_eq!(document.slides[0].title, "Intro");
        assert_eq!(document.slides[1].title, "Next");
    }

    #[test]
    fn extracts_runnable_code_cell_metadata() {
        let document = parse_course(
            "# Code\n\n```python run id=\"intro-print\" timeout=5000 session='lesson-01'\nprint(\"Hello\")\n```\n",
        )
        .expect("course should parse");

        let cell = &document.code_cells[0];
        assert_eq!(cell.id, "intro-print");
        assert_eq!(cell.language, "python");
        assert_eq!(cell.kind, CodeCellKind::Run);
        assert_eq!(cell.timeout, Some(5000));
        assert_eq!(cell.session.as_deref(), Some("lesson-01"));
        assert_eq!(
            cell.source_range,
            Some(SourceRange {
                start_line: 3,
                end_line: 5
            })
        );
        assert_eq!(cell.code, "print(\"Hello\")");
    }

    #[test]
    fn generates_default_exercise_ids_like_the_frontend() {
        let document = parse_course("# Exercise\n\n```python exercise\npass\n```\n")
            .expect("course should parse");

        let cell = &document.code_cells[0];
        assert_eq!(cell.id, "slide-1-cell-1");
        assert_eq!(cell.language, "python");
        assert_eq!(cell.kind, CodeCellKind::Exercise);
        assert_eq!(cell.timeout, None);
        assert_eq!(cell.session, None);
    }
}
