mod markdown;
mod project_package;
mod runtime;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunCodeCellRequest {
    source: String,
    cell_id: String,
    file_path: Option<String>,
    working_directory: Option<String>,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn parse_course(source: String) -> Result<markdown::CourseDocument, String> {
    markdown::parse_course(&source)
}

#[tauri::command]
async fn python_run(
    request: runtime::python::PythonRunRequest,
) -> Result<runtime::python::PythonRunResult, String> {
    tauri::async_runtime::spawn_blocking(move || runtime::python::run(request))
        .await
        .map_err(|error| format!("Python runtime task failed: {error}"))
}

#[tauri::command]
async fn run_code_cell(
    request: RunCodeCellRequest,
) -> Result<runtime::python::PythonRunResult, String> {
    tauri::async_runtime::spawn_blocking(move || run_code_cell_authoritative(request))
        .await
        .map_err(|error| format!("Code cell runtime task failed: {error}"))?
}

fn run_code_cell_authoritative(
    request: RunCodeCellRequest,
) -> Result<runtime::python::PythonRunResult, String> {
    let document = markdown::parse_course(&request.source)?;
    let cell = document
        .code_cells
        .into_iter()
        .find(|candidate| candidate.id == request.cell_id)
        .ok_or_else(|| format!("Code cell '{}' was not found.", request.cell_id))?;

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
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            parse_course,
            python_run,
            run_code_cell,
            project_package::open_project_package,
            project_package::save_project_package
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_code_cell_reparses_source_and_executes_matching_cell() {
        let result = run_code_cell_authoritative(RunCodeCellRequest {
            source:
                "# Lesson\n\n```python run id=\"hello\"\nprint('from authoritative source')\n```\n"
                    .to_string(),
            cell_id: "hello".to_string(),
            file_path: None,
            working_directory: None,
        })
        .expect("authoritative run should return a runtime result");

        if result.error.as_ref().map(|error| error.name.as_str()) == Some("PythonNotFound") {
            return;
        }

        assert_eq!(result.status, runtime::python::RunStatus::Success);
        assert_eq!(result.stdout.trim(), "from authoritative source");
    }

    #[test]
    fn run_code_cell_reports_missing_cell() {
        let result = run_code_cell_authoritative(RunCodeCellRequest {
            source: "# Lesson\n".to_string(),
            cell_id: "missing".to_string(),
            file_path: None,
            working_directory: None,
        });

        assert!(result
            .expect_err("missing code cell should be a command error")
            .contains("missing"));
    }
}
