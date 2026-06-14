import { invoke } from "@tauri-apps/api/core";
import type {
  RunCodeCellRequest,
  RunRequest,
  RunResult,
  RuntimeAdapter,
} from "./runResult";

export class PythonRuntimeClient implements RuntimeAdapter {
  readonly language = "python";

  run(request: RunRequest): Promise<RunResult> {
    assertTauriRuntime();
    return invoke<RunResult>("python_run", { request });
  }

  runCodeCell(request: RunCodeCellRequest): Promise<RunResult> {
    assertTauriRuntime();
    return invoke<RunResult>("run_code_cell", { request });
  }
}

function assertTauriRuntime() {
  if (!("__TAURI_INTERNALS__" in window)) {
    throw new Error("Python runtime is available in the Tauri desktop app.");
  }
}
