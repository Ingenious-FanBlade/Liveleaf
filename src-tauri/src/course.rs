//! Course execution shared by desktop and CLI adapters.
use crate::{markdown, runtime};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunCodeCellRequest {
    pub source: String,
    pub cell_id: String,
    pub file_path: Option<String>,
    pub working_directory: Option<String>,
}

pub(crate) fn run_code_cell(
    request: RunCodeCellRequest,
) -> Result<runtime::python::PythonRunResult, String> {
    let document = markdown::parse_course(&request.source)?;
    let mut matches = document
        .code_cells
        .into_iter()
        .filter(|candidate| candidate.id == request.cell_id);
    let cell = matches
        .next()
        .ok_or_else(|| format!("Code cell '{}' was not found.", request.cell_id))?;
    if matches.next().is_some() {
        return Err(format!(
            "Code cell ID '{}' is ambiguous; IDs must be unique.",
            request.cell_id
        ));
    }

    if let Some(diagnostics) = &cell.visualization_diagnostics {
        return Ok(runtime::python::error_result(
            cell.id,
            "VisualizationConfigError",
            diagnostics
                .iter()
                .map(|d| {
                    format!(
                        "{}: {}",
                        d.attribute.as_deref().unwrap_or("visualize"),
                        d.message
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
        ));
    }

    if cell.language.to_lowercase() != "python" {
        return Ok(runtime::python::error_result(
            cell.id,
            "UnsupportedRuntime",
            format!("No Python runtime is configured for '{}'.", cell.language),
        ));
    }

    let mode = match cell.kind {
        markdown::CodeCellKind::Run => "run",
        markdown::CodeCellKind::Exercise => "exercise",
    };

    Ok(runtime::python::run(runtime::python::PythonRunRequest {
        id: cell.id,
        code: cell.code,
        file_path: request.file_path,
        working_directory: request.working_directory,
        timeout_ms: cell.timeout.unwrap_or(5_000),
        session_id: cell.session,
        mode: mode.to_string(),
        visualization: cell.visualization,
        source_start_line: cell.source_range.map(|range| range.start_line),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_ambiguous_ids_before_starting_python() {
        let result = run_code_cell(RunCodeCellRequest {
            source: "```python run id=duplicate\nraise Exception('must not run')\n```\n\n```python run id=duplicate\npass\n```".into(),
            cell_id: "duplicate".into(), file_path: None, working_directory: None,
        });
        assert!(result.unwrap_err().contains("ambiguous"));
    }
}
