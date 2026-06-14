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
}

pub fn run(request: PythonRunRequest) -> PythonRunResult {
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

    let mut last_spawn_error = None;

    for executable in python_executable_candidates() {
        let stdout_file = File::create(&stdout_path)
            .map_err(|error| format!("Failed to create stdout capture file: {error}"))?;
        let stderr_file = File::create(&stderr_path)
            .map_err(|error| format!("Failed to create stderr capture file: {error}"))?;

        let mut command = Command::new(executable);
        command
            .arg("-u")
            .arg(&script_path)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file))
            .env("PYTHONUNBUFFERED", "1")
            .env("STREAM_LEARN_CELL_ID", &request.id)
            .env("STREAM_LEARN_RUN_MODE", &request.mode);

        if let Some(session_id) = request.session_id.as_deref() {
            command.env("STREAM_LEARN_SESSION_ID", session_id);
        }

        if let Some(file_path) = request.file_path.as_deref() {
            command.env("STREAM_LEARN_FILE_PATH", file_path);
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

        return match status {
            WaitOutcome::Exited(exit_status) if exit_status.success() => Ok(ProcessRunOutput {
                status: RunStatus::Success,
                stdout,
                stderr,
                error: None,
            }),
            WaitOutcome::Exited(exit_status) => {
                let error = parse_python_error(&stderr).or_else(|| {
                    Some(RuntimeError {
                        name: "PythonError".to_string(),
                        message: format!("Python exited with status {exit_status}."),
                        traceback: None,
                    })
                });

                Ok(ProcessRunOutput {
                    status: RunStatus::Error,
                    stdout,
                    stderr,
                    error,
                })
            }
            WaitOutcome::Timeout => Ok(ProcessRunOutput {
                status: RunStatus::Timeout,
                stdout,
                stderr,
                error: Some(RuntimeError {
                    name: "TimeoutError".to_string(),
                    message: format!("Execution timed out after {} ms.", timeout.as_millis()),
                    traceback: None,
                }),
            }),
            WaitOutcome::WaitFailed(error) => Ok(spawn_failure_output(
                "PythonWaitError",
                format!("The Python process could not be observed: {error}"),
            )),
        };
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
    let outputs = if output.stdout.is_empty() {
        Vec::new()
    } else {
        vec![RichOutput {
            kind: RichOutputKind::Text,
            mime: Some("text/plain".to_string()),
            value: output.stdout.clone(),
        }]
    };

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
        "stream-learn-python-{safe_id}-{}-{timestamp}",
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
