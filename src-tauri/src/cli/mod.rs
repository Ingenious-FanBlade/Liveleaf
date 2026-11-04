//! Headless adapters for the same course operations used by the desktop app.
mod args;
mod spec;

use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use serde::Serialize;
use serde_json::{json, Value};

use crate::{course, markdown, project_package, runtime, validation};
use args::{Args, HELP};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    schema_version: u32,
    command: String,
    ok: bool,
    data: Value,
    diagnostics: Vec<validation::Diagnostic>,
    error: Option<Failure>,
    #[serde(skip)]
    exit_code: u8,
}

#[derive(Serialize)]
struct Failure {
    code: &'static str,
    message: String,
}

impl Failure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    fn io(message: impl Into<String>) -> Self {
        Self::new("IoError", message)
    }
}

impl Response {
    fn success(command: &str, data: Value) -> Self {
        Self {
            schema_version: 1,
            command: command.into(),
            ok: true,
            data,
            diagnostics: vec![],
            error: None,
            exit_code: 0,
        }
    }
    fn failed(command: &str, failure: Failure) -> Self {
        Self {
            schema_version: 1,
            command: command.into(),
            ok: false,
            data: Value::Null,
            diagnostics: vec![],
            error: Some(failure),
            exit_code: 2,
        }
    }
}

/// No Tauri app or WebView is created on this path.
pub fn main() -> ExitCode {
    let raw_args: Vec<_> = std::env::args_os().skip(1).collect();
    let json_output = raw_args.iter().take_while(|arg| *arg != "--").any(|arg| arg == "--json");
    let command = raw_args
        .iter()
        .filter_map(|a| a.to_str())
        .find(|a| args::COMMANDS.contains(a))
        .unwrap_or("cli");
    let response = match args::parse(&raw_args) {
        Ok(options) => {
            let name = options.command.clone();
            execute(options).unwrap_or_else(|error| Response::failed(&name, error))
        }
        Err(error) => Response::failed(command, Failure::new("UsageError", error)),
    };
    if let Err(error) = emit(&response, json_output) {
        eprintln!("Failed to write CLI response: {error}");
        return ExitCode::from(2);
    }
    ExitCode::from(response.exit_code)
}

fn emit(response: &Response, json_output: bool) -> io::Result<()> {
    if json_output {
        let mut stdout = io::stdout().lock();
        serde_json::to_writer(&mut stdout, response).map_err(io::Error::other)?;
        writeln!(stdout)
    } else if let Some(help) = response.data.get("help").and_then(Value::as_str) {
        print!("{help}");
        Ok(())
    } else if let Some(version) = response.data.get("version").and_then(Value::as_str) {
        println!("liveleaf-cli {version}");
        Ok(())
    } else {
        // Human mode uses stderr for diagnostics; JSON mode always keeps one stdout document.
        for diagnostic in &response.diagnostics {
            eprintln!(
                "{} {} (line {}): {}",
                diagnostic.severity,
                diagnostic.code,
                diagnostic.line.map_or("?".into(), |n| n.to_string()),
                diagnostic.message
            );
        }
        if let Some(error) = &response.error {
            eprintln!("{}: {}", error.code, error.message);
        }
        if !response.data.is_null() {
            println!(
                "{}",
                serde_json::to_string_pretty(&response.data).map_err(io::Error::other)?
            );
        }
        Ok(())
    }
}

struct Input {
    source: String,
    path: Option<PathBuf>,
    base_dir: PathBuf,
}

fn read_input(options: &Args) -> Result<Input, Failure> {
    let (source, path, base_dir) = if options.stdin {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|e| Failure::io(format!("Cannot read UTF-8 stdin: {e}")))?;
        let base_dir = options
            .base_dir
            .clone()
            .map(Ok)
            .unwrap_or_else(std::env::current_dir)
            .map_err(|e| Failure::io(e.to_string()))?;
        (source, None, base_dir)
    } else {
        let path = options
            .input
            .as_ref()
            .expect("argument parser checks input")
            .canonicalize()
            .map_err(|e| Failure::io(format!("Cannot resolve input: {e}")))?;
        let source = fs::read_to_string(&path)
            .map_err(|e| Failure::io(format!("Cannot read '{}': {e}", path.display())))?;
        let base_dir = path
            .parent()
            .expect("absolute file has parent")
            .to_path_buf();
        (source, Some(path), base_dir)
    };
    let base_dir = base_dir
        .canonicalize()
        .map_err(|e| Failure::io(format!("Cannot resolve base directory: {e}")))?;
    if !base_dir.is_dir() {
        return Err(Failure::io("Base directory must be a directory."));
    }
    Ok(Input {
        source: source.trim_start_matches('\u{feff}').to_string(),
        path,
        base_dir,
    })
}

fn execute(options: Args) -> Result<Response, Failure> {
    if options.help {
        return Ok(Response::success("help", json!({"help": HELP})));
    }
    if options.version {
        return Ok(Response::success(
            "version",
            json!({"version": env!("CARGO_PKG_VERSION")}),
        ));
    }
    if options.command == "spec" {
        return Ok(Response::success("spec", spec::spec()));
    }

    let input = read_input(&options)?;
    let document =
        markdown::parse_course(&input.source).map_err(|e| Failure::new("ParseError", e))?;
    let report = validation::validate(&document, &input.base_dir).map_err(Failure::io)?;
    let mut response = Response::success(
        &options.command,
        json!({
        "file": input.path.as_ref().map(|path| path.to_string_lossy()),
        "baseDirectory": input.base_dir.to_string_lossy(),
            "title": document.title,
            "slideCount": document.slides.len(),
            "codeCellCount": document.code_cells.len(),
            "valid": report.valid,
            "errorCount": report.error_count,
            "warningCount": report.warning_count,
            "resources": report.resources,
        }),
    );
    response.diagnostics = report.diagnostics;

    if options.command == "inspect" {
        response.data["document"] = serde_json::to_value(&document)
            .map_err(|e| Failure::new("SerializationError", e.to_string()))?;
    }
    if !report.valid {
        response.ok = false;
        response.exit_code = 1;
        response.error = Some(Failure::new(
            "ValidationFailed",
            "Course has validation errors. No code was executed and no package was written.",
        ));
        return Ok(response);
    }
    match options.command.as_str() {
        "validate" | "inspect" => {}
        "run" => {
            let cell_id = options.cell.expect("argument parser checks cell");
            if !document.code_cells.iter().any(|c| c.id == cell_id) {
                response.ok = false;
                response.exit_code = 1;
                response.error = Some(Failure::new(
                    "CellNotFound",
                    format!("Code cell '{cell_id}' was not found."),
                ));
                return Ok(response);
            }
            let result = course::run_code_cell(course::RunCodeCellRequest {
                source: input.source,
                cell_id,
                file_path: input.path.map(|p| p.to_string_lossy().into_owned()),
                working_directory: Some(input.base_dir.to_string_lossy().into_owned()),
            })
            .map_err(|e| Failure::new("RunError", e))?;
            if result.status != runtime::python::RunStatus::Success {
                response.ok = false;
                let setup_error = result.error.as_ref().is_some_and(|e| {
                    matches!(
                        e.name.as_str(),
                        "PythonNotFound" | "PythonSpawnError" | "PythonWaitError"
                    )
                });
                response.exit_code = if setup_error { 2 } else { 1 };
                response.error = Some(Failure::new(
                    if setup_error {
                        "RuntimeSetupError"
                    } else {
                        "ExecutionFailed"
                    },
                    "The Python cell did not finish successfully; see data.result.",
                ));
            }
            response.data["result"] = serde_json::to_value(result)
                .map_err(|e| Failure::new("SerializationError", e.to_string()))?;
        }
        "pack" => {
            let result = pack(
                &input,
                options.output.expect("argument parser checks output"),
                options.force,
            )?;
            response.data["package"] = serde_json::to_value(result)
                .map_err(|e| Failure::new("SerializationError", e.to_string()))?;
        }
        _ => unreachable!("argument parser checks command"),
    }
    Ok(response)
}

fn pack(
    input: &Input,
    output: PathBuf,
    force: bool,
) -> Result<project_package::SaveProjectPackageResponse, Failure> {
    let output = std::env::current_dir()
        .map_err(|e| Failure::io(e.to_string()))?
        .join(output);
    if output.exists() && !force {
        return Err(Failure::new(
            "OutputExists",
            "Output already exists. Use --force to replace a package.",
        ));
    }
    let parent = output
        .parent()
        .ok_or_else(|| Failure::io("Output must have a parent directory."))?;
    fs::create_dir_all(parent)
        .map_err(|e| Failure::io(format!("Cannot create output directory: {e}")))?;
    let output = parent
        .canonicalize()
        .map_err(|e| Failure::io(e.to_string()))?
        .join(
            output
                .file_name()
                .ok_or_else(|| Failure::io("Output must name a file."))?,
        );
    if input.path.as_ref().is_some_and(|p| p == &output) {
        return Err(Failure::new(
            "InputOutputConflict",
            "Output must not replace the source document.",
        ));
    }
    // The package is completed beside its destination before publication.
    let temp_path = tempfile::NamedTempFile::new_in(output.parent().expect("output has parent"))
        .map_err(|e| Failure::io(format!("Cannot create temporary package: {e}")))?
        .into_temp_path();
    let mut packed = project_package::save_project_package_blocking(
        project_package::SaveProjectPackageRequest {
            source: input.source.clone(),
            output_path: temp_path.to_string_lossy().into_owned(),
            source_file_path: Some(
                input
                    .path
                    .clone()
                    .unwrap_or_else(|| input.base_dir.join("stdin.ll.md"))
                    .to_string_lossy()
                    .into_owned(),
            ),
        },
    )
    .map_err(|e| Failure::new("PackError", e))?;
    // Platform-native persistence also works on Windows volumes without hard links.
    if force {
        temp_path.persist(&output)
    } else {
        temp_path.persist_noclobber(&output)
    }
    .map_err(|e| Failure::io(format!("Cannot publish package: {}", e.error)))?;
    packed.output_path = output.to_string_lossy().into_owned();
    Ok(packed)
}
