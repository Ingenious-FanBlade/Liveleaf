//! Static authoring diagnostics. This module never executes course code.
use std::collections::{HashMap, HashSet};
use std::path::Path;

use ::markdown::{mdast::Node, to_mdast, ParseOptions};
use serde::Serialize;

use crate::markdown::{self, CourseDocument};
use crate::project_package::{self, ResourceInfo};
use crate::runtime::visualization;

pub(crate) const DIRECTIVES: &[&str] = &[
    "note", "tip", "warning", "danger", "question", "hint", "solution", "columns", "column",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Diagnostic {
    pub severity: &'static str,
    pub code: &'static str,
    pub message: String,
    pub line: Option<usize>,
    pub cell_id: Option<String>,
    pub attribute: Option<String>,
    pub resource: Option<String>,
}

impl Diagnostic {
    fn error(code: &'static str, message: impl Into<String>, line: Option<usize>) -> Self {
        Self {
            severity: "error",
            code,
            message: message.into(),
            line,
            cell_id: None,
            attribute: None,
            resource: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Report {
    pub valid: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub resources: Vec<ResourceInfo>,
}

pub(crate) fn validate(document: &CourseDocument, base_dir: &Path) -> Result<Report, String> {
    let mut diagnostics = Vec::new();
    let mut ids = HashMap::new();
    for cell in &document.code_cells {
        let line = cell.source_range.as_ref().map(|r| r.start_line);
        if cell.id.trim().is_empty() {
            let mut diagnostic =
                Diagnostic::error("EmptyCellId", "Code cell IDs must not be empty.", line);
            diagnostic.cell_id = Some(cell.id.clone());
            diagnostics.push(diagnostic);
        }
        if let Some(first_line) = ids.insert(&cell.id, line) {
            let mut diagnostic = Diagnostic::error(
                "DuplicateCellId",
                format!(
                    "Duplicate code cell ID '{}'; first occurrence at line {}.",
                    cell.id,
                    first_line.unwrap_or(1)
                ),
                line,
            );
            diagnostic.cell_id = Some(cell.id.clone());
            diagnostics.push(diagnostic);
        }
        if !cell.language.eq_ignore_ascii_case("python") {
            let mut diagnostic = Diagnostic::error(
                "UnsupportedRuntime",
                format!(
                    "Runnable cells require python; '{}' is supported only as static code.",
                    cell.language
                ),
                line,
            );
            diagnostic.cell_id = Some(cell.id.clone());
            diagnostics.push(diagnostic);
        }
        for issue in cell.visualization_diagnostics.iter().flatten() {
            let mut diagnostic =
                Diagnostic::error("VisualizationConfigError", &issue.message, line);
            diagnostic.cell_id = Some(cell.id.clone());
            diagnostic.attribute = issue.attribute.clone();
            diagnostics.push(diagnostic);
        }
    }

    let ast = to_mdast(&document.source, &ParseOptions::gfm()).map_err(|e| e.to_string())?;
    // mdast may end an unclosed fence at the empty line after a trailing newline.
    let lines: Vec<_> = document.source.split('\n').collect();
    validate_code(&ast, &lines, document, &mut diagnostics);
    validate_directives(&lines, &markdown::code_lines(&ast), &mut diagnostics);
    let (resources, resource_warnings) =
        project_package::inspect_resources(&document.source, base_dir)?;
    for resource in &resources {
        let (code, message, severity) = match resource.status.as_str() {
            "missing" => (
                "MissingResource",
                "Local resource is missing or is not a file.",
                "error",
            ),
            "blocked" => (
                "BlockedResource",
                "Resource lies outside the project directory and cannot be bundled.",
                "error",
            ),
            "external" => (
                "ExternalResource",
                "External resource will remain a URL; availability was not checked.",
                "warning",
            ),
            _ => continue,
        };
        let line = lines
            .iter()
            .position(|line| line.contains(&resource.source))
            .map(|i| i + 1);
        let mut diagnostic = Diagnostic::error(code, message, line);
        diagnostic.severity = severity;
        diagnostic.resource = Some(resource.source.clone());
        diagnostics.push(diagnostic);
    }
    // Other manifest warnings have no corresponding asset (e.g. an empty URL).
    for warning in resource_warnings
        .iter()
        .filter(|w| w.contains("empty resource"))
    {
        let mut diagnostic = Diagnostic::error("EmptyResource", warning, None);
        diagnostic.severity = "warning";
        diagnostics.push(diagnostic);
    }
    diagnostics.sort_by(|a, b| {
        (a.line, a.code, &a.attribute, &a.message).cmp(&(b.line, b.code, &b.attribute, &b.message))
    });
    let error_count = diagnostics.iter().filter(|d| d.severity == "error").count();
    let warning_count = diagnostics.len() - error_count;
    Ok(Report {
        valid: error_count == 0,
        error_count,
        warning_count,
        diagnostics,
        resources,
    })
}

fn validate_code(
    node: &Node,
    lines: &[&str],
    document: &CourseDocument,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Node::Code(code) = node {
        if let Some(position) = &code.position {
            let start = position.start.line;
            let opening: String = lines[start - 1]
                .chars()
                .skip(position.start.column - 1)
                .collect();
            let marker = opening.chars().next().unwrap_or(' ');
            let length = opening.chars().take_while(|c| *c == marker).count();
            if matches!(marker, '`' | '~') && length >= 3 {
                let closing = lines[position.end.line - 1].trim_start_matches([' ', '\t', '>']);
                let closing_length = closing.chars().take_while(|c| *c == marker).count();
                if position.end.line == start
                    || closing_length < length
                    || !closing[closing_length..].trim().is_empty()
                {
                    diagnostics.push(Diagnostic::error(
                        "UnclosedCodeFence",
                        "Code fence has no matching closing fence.",
                        Some(start),
                    ));
                }
            }

            let params = markdown::parse_params(code.meta.as_deref().unwrap_or(""));
            if params.flags.contains("run")
                || params.flags.contains("exercise")
                || params.attrs.contains_key("run")
                || params.attrs.contains_key("exercise")
            {
                let cell_id = document
                    .code_cells
                    .iter()
                    .find(|cell| {
                        cell.source_range
                            .as_ref()
                            .is_some_and(|r| r.start_line == start)
                    })
                    .map(|c| c.id.clone());
                let mut push = |code, message: String, attribute: Option<String>| {
                    let mut d = Diagnostic::error(code, message, Some(start));
                    d.cell_id = cell_id.clone();
                    d.attribute = attribute;
                    diagnostics.push(d);
                };
                if params.unterminated_quote {
                    push(
                        "UnterminatedAttribute",
                        "A quoted code attribute has no closing quote.".into(),
                        None,
                    );
                }
                if params.flags.contains("run") && params.flags.contains("exercise") {
                    push(
                        "ConflictingCellKind",
                        "Choose either run or exercise, not both.".into(),
                        None,
                    );
                }
                for attribute in ["id", "timeout", "session"] {
                    if params.duplicates.contains(attribute) {
                        push(
                            "DuplicateAttribute",
                            format!("Duplicate attribute: {attribute}"),
                            Some(attribute.into()),
                        );
                    }
                    if params.flags.contains(attribute) {
                        push(
                            "MissingAttributeValue",
                            format!("{attribute} requires a value."),
                            Some(attribute.into()),
                        );
                    }
                }
                if let Some(timeout) = params.attrs.get("timeout") {
                    if !timeout
                        .parse::<u64>()
                        .is_ok_and(|t| (1..=3_600_000).contains(&t))
                    {
                        push(
                            "InvalidTimeout",
                            "timeout must be an integer from 1 to 3600000 milliseconds.".into(),
                            Some("timeout".into()),
                        );
                    }
                }
                for name in params.attrs.keys().chain(params.flags.iter()) {
                    if !["run", "exercise", "id", "timeout", "session"].contains(&name.as_str())
                        && !visualization::ATTRIBUTES.contains(&name.as_str())
                    {
                        push(
                            "UnknownCodeAttribute",
                            format!("Unsupported runnable-code attribute: {name}"),
                            Some(name.clone()),
                        );
                    }
                }
                for name in ["run", "exercise"] {
                    if params.attrs.contains_key(name) {
                        push(
                            "InvalidCellKind",
                            format!("{name} is a flag and must not have a value."),
                            Some(name.into()),
                        );
                    }
                }
            }
        }
    }
    if let Some(children) = node.children() {
        for child in children {
            validate_code(child, lines, document, diagnostics);
        }
    }
}

fn validate_directives(
    lines: &[&str],
    code_lines: &HashSet<usize>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut stack: Vec<(&str, usize)> = Vec::new();
    for (index, raw) in lines.iter().enumerate() {
        let line_number = index + 1;
        if code_lines.contains(&line_number) || raw.len() - raw.trim_start().len() > 3 {
            continue;
        }
        let line = raw.trim();
        if line == "---" {
            for (name, start) in stack.drain(..) {
                diagnostics.push(Diagnostic::error(
                    "UnclosedDirective",
                    format!("{name} must close before the slide separator."),
                    Some(start),
                ));
            }
        } else if line == ":::" {
            if stack.pop().is_none() {
                diagnostics.push(Diagnostic::error(
                    "UnexpectedDirectiveClose",
                    "No open directive matches this closing :::.",
                    Some(line_number),
                ));
            }
        } else if let Some(rest) = line.strip_prefix(":::") {
            let name = rest.split_whitespace().next().unwrap_or("");
            if !DIRECTIVES.contains(&name) {
                diagnostics.push(Diagnostic::error(
                    "UnknownDirective",
                    format!("Unsupported directive: {name}"),
                    Some(line_number),
                ));
            }
            if name == "column" && stack.last().map(|v| v.0) != Some("columns") {
                diagnostics.push(Diagnostic::error(
                    "InvalidColumn",
                    "A column must be a direct child of columns.",
                    Some(line_number),
                ));
            }
            stack.push((name, line_number));
        }
    }
    for (name, line) in stack {
        diagnostics.push(Diagnostic::error(
            "UnclosedDirective",
            format!("{name} has no closing :::."),
            Some(line),
        ));
    }
}
