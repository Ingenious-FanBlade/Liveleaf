# Python Runtime Design

## 1. Goal

The first executable runtime in Liveleaf is Python.

The Python runtime allows course authors to write runnable Python code blocks in
Markdown and allows teachers or students to execute those blocks inside the
application.

Example:

````markdown
```python run
print("Hello, Liveleaf")
```
````

## 2. Runtime Scope

MVP requirements:

- Execute Python code blocks marked with `run`.
- Execute Python exercise blocks marked with `exercise`.
- Capture stdout.
- Capture stderr.
- Capture exceptions and tracebacks.
- Enforce a timeout.
- Return a normalized run result.
- Display the result below the code block.

Out of scope for the first implementation:

- Full Jupyter protocol compatibility.
- Remote kernels.
- Package management UI.
- Notebook checkpointing.
- GPU support.
- Multi-user execution.
- Online judge service.

## 3. Recommended MVP Runtime Mode

Use local Python execution through the Tauri backend.

Reason:

- It fits the desktop application model.
- It supports real Python semantics.
- It can use packages installed on the user's machine.
- It avoids shipping a large WebAssembly runtime in the first version.

Tradeoffs:

- The user must have Python installed, or the app must later bundle Python.
- Local execution has security risks.
- Package availability differs across machines.

Future runtime modes:

- Bundled Python sidecar.
- Pyodide in a Web Worker.
- Remote classroom runtime.
- Containerized sandbox.

## 4. Runtime Architecture

```text
Vue UI
  |
  | invoke("python_run", request)
  v
Tauri Command
  |
  | PythonRuntime
  v
Python Process
  |
  | stdout, stderr, exit code, metadata
  v
Normalized RunResult
  |
  v
Vue Output Renderer
```

## 5. Runtime Adapter Interface

Frontend interface:

```ts
export interface RuntimeAdapter {
  language: string;
  run(request: RunRequest): Promise<RunResult>;
  interrupt?(sessionId: string): Promise<void>;
}

export interface RunRequest {
  id: string;
  language: "python";
  code: string;
  filePath?: string;
  workingDirectory?: string;
  timeoutMs: number;
  sessionId?: string;
  mode: "run" | "exercise";
}
```

Normalized result:

```ts
export interface RunResult {
  id: string;
  language: "python";
  status: "success" | "error" | "timeout" | "cancelled";
  stdout: string;
  stderr: string;
  outputs: RichOutput[];
  startedAt: string;
  endedAt: string;
  durationMs: number;
  error?: RuntimeError;
}

export interface RichOutput {
  kind: "text" | "html" | "markdown" | "image" | "json";
  mime?: string;
  value: string;
}

export interface RuntimeError {
  name: string;
  message: string;
  traceback?: string;
}
```

## 6. Tauri Command Design

Suggested command:

```rust
#[tauri::command]
async fn python_run(request: PythonRunRequest) -> Result<PythonRunResult, String> {
    // Implementation delegated to runtime::python.
}
```

Suggested Rust request model:

```rust
#[derive(Debug, serde::Deserialize)]
pub struct PythonRunRequest {
    pub id: String,
    pub code: String,
    pub file_path: Option<String>,
    pub working_directory: Option<String>,
    pub timeout_ms: u64,
    pub session_id: Option<String>,
    pub mode: String,
}
```

## 7. Execution Strategy

### 7.1 MVP Strategy: Process Per Run

For the first implementation, start a Python process for each run.

Benefits:

- Simpler to implement.
- Easier to enforce timeout.
- State does not leak between runs.
- A failed run does not poison a long-lived session.

Limitations:

- Variables do not persist across code blocks.
- Startup cost is paid each time.
- Less notebook-like.

This is acceptable for the first vertical slice because it proves Markdown code
execution, output capture, and UI integration.

### 7.2 Next Strategy: Session Kernel

After the MVP works, add optional session-scoped execution.

Example syntax:

````markdown
```python run session="lesson-01"
x = 10
```

```python run session="lesson-01"
print(x)
```
````

Expected behavior:

- Blocks with the same session share Python state.
- A session can be restarted.
- A session can be interrupted.
- The UI shows whether a block belongs to a session.

Implementation options:

- Maintain a long-running Python process per session.
- Use a simple JSON protocol over stdin and stdout.
- Later consider Jupyter kernel protocol if needed.

## 8. Output Capture

MVP output types:

- Plain text stdout.
- Plain text stderr.
- Exception traceback.

Future output types:

- HTML.
- Markdown.
- JSON.
- PNG images.
- SVG images.
- Matplotlib figures.

For future rich output, the runtime can inject a small helper into the executed
Python code. For MVP, plain process output is enough.

## 9. Error Handling

The runtime should distinguish:

- Python exception.
- Timeout.
- Python executable not found.
- Process spawn failure.
- Permission failure.
- Internal runtime failure.

User-facing messages should be direct and actionable.

Examples:

- `Python was not found. Configure a Python executable in Settings.`
- `Execution timed out after 5000 ms.`
- `The process failed to start. Check runtime settings.`

## 10. Timeout And Cancellation

MVP behavior:

- Every run request has a timeout.
- Default timeout: 5000 ms.
- The backend kills the child process when timeout is reached.
- The returned status is `timeout`.

Future behavior:

- Manual interrupt button.
- Per-block timeout.
- Per-course runtime defaults.
- Long-running session interrupt.

## 11. Working Directory

MVP default:

- Execute in a temporary directory created for the run.

Alternative when needed:

- Execute relative to the course file directory.

Recommended rule:

- Normal examples use a temporary directory.
- Blocks that need course assets can opt into the course directory later.

Future syntax:

````markdown
```python run cwd="course"
from pathlib import Path
print(Path("assets/data.csv").exists())
```
````

## 12. Python Executable Resolution

MVP lookup order:

1. User-configured Python path.
2. `python` on Windows.
3. `python3` on Unix-like systems.
4. Future bundled Python path.

Settings should eventually expose:

- Python executable path.
- Default timeout.
- Whether to use temporary directory or course directory.

## 13. Security Notes

Python code should be treated as local executable code.

MVP safety decisions:

- Do not auto-run code when opening a file.
- Run only after explicit user action.
- Display local execution status in the UI.
- Use timeouts.
- Keep output files separate from source.

Future safety work:

- Trust model for downloaded course files.
- Per-course permission prompts.
- Sandboxed execution mode.
- Container-based execution.
- Network restriction where possible.

## 14. Suggested Development Milestones

### Milestone 1: Frontend Runtime Contract

- Define `RunRequest`.
- Define `RunResult`.
- Add a simple Python runtime client.
- Add a code block run button.
- Show pending, success, error, and timeout states.

### Milestone 2: Tauri Python Command

- Add `python_run` command.
- Spawn Python process.
- Pass code through stdin or a temporary file.
- Capture stdout and stderr.
- Return normalized result.

### Milestone 3: Markdown Integration

- Parse code fence metadata.
- Detect `python run`.
- Render executable code block component.
- Connect run button to runtime client.

### Milestone 4: Output Persistence

- Save latest run result in `.liveleaf/outputs.json`.
- Restore outputs when reopening the course.
- Allow clearing outputs.

### Milestone 5: Session Runtime

- Add `session` attribute support.
- Keep session-scoped Python process.
- Support restart and interrupt.

