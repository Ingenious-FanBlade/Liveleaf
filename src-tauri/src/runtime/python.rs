use super::visualization::{self, VisualizationConfig};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_TIMEOUT_MS: u64 = 5_000;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonRunRequest {
    pub id: String,
    pub code: String,
    pub file_path: Option<String>,
    pub working_directory: Option<String>,
    pub timeout_ms: u64,
    pub session_id: Option<String>,
    pub mode: String,
    pub visualization: Option<VisualizationConfig>,
    #[serde(skip)]
    pub source_start_line: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PythonRunResult {
    pub id: String,
    pub language: String,
    pub status: RunStatus,
    pub stdout: String,
    pub stderr: String,
    pub outputs: Vec<RichOutput>,
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: u64,
    pub error: Option<RuntimeError>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RunStatus {
    Success,
    Error,
    Timeout,
    Cancelled,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RichOutput {
    pub kind: RichOutputKind,
    pub mime: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RichOutputKind {
    Text,
    Html,
    Markdown,
    Image,
    Json,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeError {
    pub name: String,
    pub message: String,
    pub traceback: Option<String>,
}

struct RunClock {
    started_system: SystemTime,
    started_instant: Instant,
}

struct ProcessRunOutput {
    status: RunStatus,
    stdout: String,
    stderr: String,
    error: Option<RuntimeError>,
    outputs: Vec<RichOutput>,
}

pub fn run(request: PythonRunRequest) -> PythonRunResult {
    if let Some(config) = &request.visualization {
        if let Err(message) = visualization::validate_config(config) {
            return error_result(request.id, "VisualizationConfigError", message);
        }
    }
    let clock = RunClock::start();
    let timeout_ms = if request.timeout_ms == 0 {
        DEFAULT_TIMEOUT_MS
    } else {
        request.timeout_ms
    }
    .min(60 * 60 * 1_000);
    let run_output = match run_process(&request, Duration::from_millis(timeout_ms)) {
        Ok(output) => output,
        Err(error) => ProcessRunOutput {
            outputs: vec![],
            status: RunStatus::Error,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(RuntimeError {
                name: "RuntimeError".to_string(),
                message: error,
                traceback: None,
            }),
        },
    };

    build_result(request.id, clock, run_output)
}

pub fn error_result(id: String, name: &str, message: String) -> PythonRunResult {
    build_result(
        id,
        RunClock::start(),
        ProcessRunOutput {
            outputs: vec![],
            status: RunStatus::Error,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(RuntimeError {
                name: name.to_string(),
                message,
                traceback: None,
            }),
        },
    )
}

fn run_process(request: &PythonRunRequest, timeout: Duration) -> Result<ProcessRunOutput, String> {
    let run_dir = create_temp_run_dir(&request.id)?;
    let output = run_process_in_dir(request, timeout, &run_dir);
    let _ = fs::remove_dir_all(&run_dir);

    output
}

fn run_process_in_dir(
    request: &PythonRunRequest,
    timeout: Duration,
    run_dir: &Path,
) -> Result<ProcessRunOutput, String> {
    let script_path = run_dir.join("cell.py");
    let stdout_path = run_dir.join("stdout.txt");
    let stderr_path = run_dir.join("stderr.txt");

    fs::write(&script_path, &request.code)
        .map_err(|error| format!("Failed to write temporary Python file: {error}"))?;

    let run_id = run_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    if let Some(config) = &request.visualization {
        visualization::write_runner(run_dir)?;
        let header = serde_json::json!({ "schemaVersion": 1, "runId": run_id, "cellId": request.id, "config": config });
        fs::write(
            run_dir.join("visualization-config.json"),
            serde_json::to_vec(&header).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }

    let mut last_spawn_error = None;

    for executable in python_executable_candidates() {
        let stdout_file = File::create(&stdout_path)
            .map_err(|error| format!("Failed to create stdout capture file: {error}"))?;
        let stderr_file = File::create(&stderr_path)
            .map_err(|error| format!("Failed to create stderr capture file: {error}"))?;

        let mut command = Command::new(executable);
        command
            .arg("-u")
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file))
            .env("PYTHONUNBUFFERED", "1")
            .env("PYTHONIOENCODING", "utf-8")
            .env("LIVELEAF_CELL_ID", &request.id)
            .env("LIVELEAF_RUN_MODE", &request.mode);

        if request.visualization.is_some() {
            command
                .arg(run_dir.join("runner.py"))
                .arg(&script_path)
                .arg(run_dir.join("visualization-config.json"))
                .arg(run_dir.join("trace.jsonl"));
        } else {
            command.arg(&script_path);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        if let Some(session_id) = request.session_id.as_deref() {
            command.env("LIVELEAF_SESSION_ID", session_id);
        }

        if let Some(file_path) = request.file_path.as_deref() {
            command.env("LIVELEAF_FILE_PATH", file_path);
        }

        command.current_dir(resolve_working_directory(request, run_dir)?);

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                last_spawn_error = Some(error.to_string());
                continue;
            }
            Err(error) => {
                return Ok(spawn_failure_output(
                    "PythonSpawnError",
                    format!("The Python process failed to start: {error}"),
                ));
            }
        };

        let status = wait_with_timeout(&mut child, timeout);
        let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
        let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();

        let mut output = match status {
            WaitOutcome::Exited(exit_status) if exit_status.success() => ProcessRunOutput {
                outputs: vec![],
                status: RunStatus::Success,
                stdout,
                stderr,
                error: None,
            },
            WaitOutcome::Exited(exit_status) => {
                let error = parse_python_error(&stderr).or_else(|| {
                    Some(RuntimeError {
                        name: "PythonError".to_string(),
                        message: format!("Python exited with status {exit_status}."),
                        traceback: None,
                    })
                });

                ProcessRunOutput {
                    outputs: vec![],
                    status: RunStatus::Error,
                    stdout,
                    stderr,
                    error,
                }
            }
            WaitOutcome::Timeout => ProcessRunOutput {
                outputs: vec![],
                status: RunStatus::Timeout,
                stdout,
                stderr,
                error: Some(RuntimeError {
                    name: "TimeoutError".to_string(),
                    message: format!("Execution timed out after {} ms.", timeout.as_millis()),
                    traceback: None,
                }),
            },
            WaitOutcome::WaitFailed(error) => spawn_failure_output(
                "PythonWaitError",
                format!("The Python process could not be observed: {error}"),
            ),
        };
        if let Some(config) = &request.visualization {
            let status = match output.status {
                RunStatus::Success => "success",
                RunStatus::Timeout => "timeout",
                _ => "error",
            };
            let trace = visualization::read_trace(
                &run_dir.join("trace.jsonl"),
                &run_id,
                &request.id,
                config,
                request.code.lines().count(),
                status,
                request.source_start_line,
            );
            output.outputs.push(RichOutput {
                kind: RichOutputKind::Json,
                mime: Some(visualization::MIME.into()),
                value: serde_json::to_string(&trace).map_err(|e| e.to_string())?,
            });
        }
        return Ok(output);
    }

    Ok(spawn_failure_output(
        "PythonNotFound",
        last_spawn_error.map_or_else(
            || "Python was not found. Configure a Python executable in Settings.".to_string(),
            |_| "Python was not found. Configure a Python executable in Settings.".to_string(),
        ),
    ))
}

fn resolve_working_directory<'a>(
    request: &'a PythonRunRequest,
    run_dir: &'a Path,
) -> Result<&'a Path, String> {
    match request.working_directory.as_deref() {
        Some(path) if !path.trim().is_empty() => Ok(Path::new(path)),
        _ => Ok(run_dir),
    }
}

enum WaitOutcome {
    Exited(std::process::ExitStatus),
    Timeout,
    WaitFailed(String),
}

fn wait_with_timeout(child: &mut std::process::Child, timeout: Duration) -> WaitOutcome {
    let deadline = Instant::now() + timeout;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return WaitOutcome::Exited(status),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return WaitOutcome::Timeout;
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => return WaitOutcome::WaitFailed(error.to_string()),
        }
    }
}

fn build_result(id: String, clock: RunClock, output: ProcessRunOutput) -> PythonRunResult {
    let ended_system = SystemTime::now();
    let mut outputs = if output.stdout.is_empty() {
        Vec::new()
    } else {
        vec![RichOutput {
            kind: RichOutputKind::Text,
            mime: Some("text/plain".to_string()),
            value: output.stdout.clone(),
        }]
    };
    outputs.extend(output.outputs);

    PythonRunResult {
        id,
        language: "python".to_string(),
        status: output.status,
        stdout: output.stdout,
        stderr: output.stderr,
        outputs,
        started_at: timestamp_string(clock.started_system),
        ended_at: timestamp_string(ended_system),
        duration_ms: clock.started_instant.elapsed().as_millis() as u64,
        error: output.error,
    }
}

fn spawn_failure_output(name: &str, message: String) -> ProcessRunOutput {
    ProcessRunOutput {
        outputs: vec![],
        status: RunStatus::Error,
        stdout: String::new(),
        stderr: String::new(),
        error: Some(RuntimeError {
            name: name.to_string(),
            message,
            traceback: None,
        }),
    }
}

fn parse_python_error(stderr: &str) -> Option<RuntimeError> {
    let trimmed = stderr.trim();

    if trimmed.is_empty() {
        return None;
    }

    let last_line = trimmed
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())?;

    let (name, message) = last_line
        .split_once(": ")
        .map_or(("PythonError", last_line), |(name, message)| {
            (name, message)
        });

    Some(RuntimeError {
        name: name.to_string(),
        message: message.to_string(),
        traceback: Some(trimmed.to_string()),
    })
}

fn create_temp_run_dir(id: &str) -> Result<PathBuf, String> {
    let mut base = std::env::temp_dir();
    let safe_id = sanitize_path_part(id);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    base.push(format!(
        "liveleaf-python-{safe_id}-{}-{timestamp}",
        std::process::id()
    ));
    fs::create_dir_all(&base)
        .map_err(|error| format!("Failed to create temporary run directory: {error}"))?;

    Ok(base)
}

fn sanitize_path_part(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();

    if sanitized.is_empty() {
        "cell".to_string()
    } else {
        sanitized
    }
}

fn timestamp_string(time: SystemTime) -> String {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| format!("{}.{:03}Z", duration.as_secs(), duration.subsec_millis()))
        .unwrap_or_else(|_| "0.000Z".to_string())
}

fn python_executable_candidates() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &["python", "py"]
    }

    #[cfg(not(windows))]
    {
        &["python3", "python"]
    }
}

impl RunClock {
    fn start() -> Self {
        Self {
            started_system: SystemTime::now(),
            started_instant: Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visual_run(code: &str, config: serde_json::Value, timeout_ms: u64) -> PythonRunResult {
        run(PythonRunRequest {
            id: "visual-test".into(),
            code: code.into(),
            file_path: None,
            working_directory: None,
            timeout_ms,
            session_id: None,
            mode: "run".into(),
            visualization: Some(serde_json::from_value(config).unwrap()),
            source_start_line: None,
        })
    }

    fn trace(result: &PythonRunResult) -> serde_json::Value {
        let output = result
            .outputs
            .iter()
            .find(|o| o.mime.as_deref() == Some(visualization::MIME))
            .expect("visualization output");
        serde_json::from_str(&output.value).unwrap()
    }

    #[test]
    fn collects_and_validates_array_and_tree_examples() {
        if !python_is_available() {
            return;
        }
        let course = crate::markdown::parse_course(include_str!(
            "../../../docs/examples/python-visualization.ll.md"
        ))
        .unwrap();
        for cell in course.code_cells {
            let config = serde_json::to_value(cell.visualization.unwrap()).unwrap();
            let result = visual_run(&cell.code, config, 5000);
            assert_eq!(result.status, RunStatus::Success, "{:?}", result.error);
            let trace = trace(&result);
            assert_eq!(trace["capture"]["status"], "complete", "{trace}");
            assert!(!trace["frames"].as_array().unwrap().is_empty());
            if cell.id == "climb-stairs" {
                assert_eq!(result.stdout.trim(), "8");
                assert_eq!(
                    trace["frames"].as_array().unwrap().last().unwrap()["state"]["items"],
                    serde_json::json!([1, 1, 2, 3, 5, 8])
                );
            } else {
                assert_eq!(result.stdout.trim(), "A → B → D → E → C → F");
            }
        }
    }

    #[test]
    fn collects_and_validates_directed_graph_and_dijkstra_examples() {
        if !python_is_available() {
            return;
        }
        let course =
            crate::markdown::parse_course(include_str!("../../../docs/examples/dijkstra.ll.md"))
                .unwrap();
        for cell in course.code_cells {
            let config = serde_json::to_value(cell.visualization.unwrap()).unwrap();
            let result = visual_run(&cell.code, config, 5000);
            assert_eq!(result.status, RunStatus::Success, "{:?}", result.error);
            let trace = trace(&result);
            assert_eq!(trace["capture"]["status"], "complete", "{trace}");
            let frames = trace["frames"].as_array().unwrap();
            assert!(!frames.is_empty());
            let last = frames.last().unwrap();
            assert_eq!(
                last["type"],
                if cell.id == "dynamic-table" {
                    "dynamic-table"
                } else {
                    "graph"
                }
            );
            if cell.id == "dijkstra-shortest-path" {
                assert!(result.stdout.contains("到 F 的路径: A → C → B → D → E → F"));
                let nodes = last["state"]["nodes"].as_array().unwrap();
                let f = nodes.iter().find(|n| n["value"] == "F").unwrap();
                assert_eq!(f["state"]["distance"], 8);
                let g = nodes.iter().find(|n| n["value"] == "G").unwrap();
                assert_eq!(
                    g["state"]["distance"],
                    serde_json::json!({"kind":"float","text":"inf"})
                );
                assert_eq!(g["state"]["visited"], false);
                assert_eq!(
                    last["tables"][0]["state"]["rows"][5],
                    serde_json::json!(["F", 8, true, "E"])
                );
            }
        }
    }

    #[test]
    fn graph_timeout_and_exception_preserve_valid_prefix() {
        if !python_is_available() {
            return;
        }
        let config = serde_json::json!({"type":"graph","watch":"graph","pointers":[],"scope":"module","graph":{}});
        let result = visual_run(
            "import time\ngraph={'A':{'B':1}}\ntime.sleep(10)\n",
            config.clone(),
            500,
        );
        assert_eq!(result.status, RunStatus::Timeout);
        let t = trace(&result);
        assert_eq!(t["capture"]["status"], "truncated");
        assert_eq!(t["capture"]["reason"], "timeout");
        assert!(!t["frames"].as_array().unwrap().is_empty());
        let result = visual_run("graph={'A':{}}\nraise ValueError('bad')\n", config, 5000);
        assert_eq!(result.status, RunStatus::Error);
        assert_eq!(trace(&result)["capture"]["status"], "complete");
    }

    #[test]
    fn timeout_recovers_flushed_prefix_and_exception_keeps_trace() {
        if !python_is_available() {
            return;
        }
        let config =
            serde_json::json!({"type":"array", "watch":"arr", "pointers":[], "scope":"module"});
        let result = visual_run(
            "import time\narr=[1]\ntime.sleep(10)\n",
            config.clone(),
            400,
        );
        assert_eq!(result.status, RunStatus::Timeout);
        let t = trace(&result);
        assert_eq!(t["capture"]["status"], "truncated");
        assert_eq!(t["capture"]["reason"], "timeout");
        assert!(!t["frames"].as_array().unwrap().is_empty());
        let result = visual_run("arr=[1]\nraise ValueError('bad')\n", config, 5000);
        assert_eq!(result.status, RunStatus::Error);
        assert_eq!(result.error.as_ref().unwrap().name, "ValueError");
        assert_eq!(trace(&result)["capture"]["status"], "complete");
    }

    #[test]
    fn direct_run_rejects_invalid_configuration_without_execution() {
        let result = visual_run(
            "print('must not run')",
            serde_json::json!({"type":"array", "watch":"arr[0]", "pointers":[], "scope":"module"}),
            5000,
        );
        assert_eq!(result.error.unwrap().name, "VisualizationConfigError");
        assert!(result.stdout.is_empty());
    }

    #[test]
    fn parses_traceback_error_name_and_message() {
        let error = parse_python_error(
            "Traceback (most recent call last):\n  File \"cell.py\", line 1, in <module>\nValueError: bad value\n",
        )
        .expect("stderr should produce runtime error");

        assert_eq!(error.name, "ValueError");
        assert_eq!(error.message, "bad value");
        assert!(error.traceback.expect("traceback").contains("Traceback"));
    }

    #[test]
    fn sanitizes_cell_ids_for_temp_paths() {
        assert_eq!(sanitize_path_part("slide-1:cell/2"), "slide-1-cell-2");
        assert_eq!(sanitize_path_part(""), "cell");
    }

    #[test]
    fn runs_python_when_available() {
        if !python_is_available() {
            return;
        }

        let result = run(PythonRunRequest {
            id: "unit-python".to_string(),
            code: "print('hello from python')".to_string(),
            file_path: None,
            working_directory: None,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            session_id: None,
            mode: "run".to_string(),
            visualization: None,
            source_start_line: None,
        });

        assert_eq!(result.status, RunStatus::Success);
        assert_eq!(result.stdout.trim(), "hello from python");
        assert!(result.stderr.is_empty());
    }

    #[test]
    fn reports_timeout_when_python_is_available() {
        if !python_is_available() {
            return;
        }

        let result = run(PythonRunRequest {
            id: "unit-timeout".to_string(),
            code: "while True:\n    pass".to_string(),
            file_path: None,
            working_directory: None,
            timeout_ms: 50,
            session_id: None,
            mode: "run".to_string(),
            visualization: None,
            source_start_line: None,
        });

        assert_eq!(result.status, RunStatus::Timeout);
        assert_eq!(
            result.error.as_ref().map(|error| error.name.as_str()),
            Some("TimeoutError")
        );
    }

    fn python_is_available() -> bool {
        python_executable_candidates().iter().any(|candidate| {
            Command::new(candidate)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
        })
    }
}
