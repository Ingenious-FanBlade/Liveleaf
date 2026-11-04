pub mod cli;
mod course;
mod validation;
mod markdown;
mod project_package;
mod runtime;

use course::{run_code_cell as run_code_cell_authoritative, RunCodeCellRequest};

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
    fn authoritative_visualization_adds_global_document_line() {
        let source = "# First\n\n---\n\n# Second\n\n```python run id=lines visualize=array watch=arr\n\narr=[1]\n```";
        let result = run_code_cell_authoritative(RunCodeCellRequest {
            source: source.into(),
            cell_id: "lines".into(),
            file_path: None,
            working_directory: None,
        })
        .unwrap();
        if result.error.as_ref().map(|e| e.name.as_str()) == Some("PythonNotFound") {
            return;
        }
        assert_eq!(result.status, runtime::python::RunStatus::Success);
        let trace: serde_json::Value = serde_json::from_str(
            &result
                .outputs
                .iter()
                .find(|o| o.mime.as_deref() == Some(runtime::visualization::MIME))
                .unwrap()
                .value,
        )
        .unwrap();
        assert_eq!(trace["capture"]["status"], "complete");
        for frame in trace["frames"].as_array().unwrap() {
            assert_eq!(
                frame["position"]["documentLine"].as_u64().unwrap(),
                7 + frame["position"]["cellLine"].as_u64().unwrap()
            );
        }
    }

    #[test]
    fn configuration_diagnostics_prevent_user_code_execution() {
        let result = run_code_cell_authoritative(RunCodeCellRequest { source: "```python run id=bad visualize=array watch=arr watch=other\nprint('must not run')\n```".into(), cell_id: "bad".into(), file_path: None, working_directory: None }).unwrap();
        assert_eq!(result.error.unwrap().name, "VisualizationConfigError");
        assert!(result.stdout.is_empty());
    }

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
