import type { VisualizationConfig } from "./visualization";

export interface RuntimeAdapter {
  language: "python";
  run(request: RunRequest): Promise<RunResult>;
  runCodeCell(request: RunCodeCellRequest): Promise<RunResult>;
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
  visualization?: VisualizationConfig;
}

export interface RunCodeCellRequest {
  source: string;
  cellId: string;
  filePath?: string;
  workingDirectory?: string;
}

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
