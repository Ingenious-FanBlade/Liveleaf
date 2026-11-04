import type { VisualizationConfig, VisualizationDiagnostic } from "../runtime/visualization";

export interface SourceRange {
  startLine: number;
  endLine: number;
}

export interface CodeCell {
  id: string;
  language: string;
  kind: "run" | "exercise";
  code: string;
  timeout?: number;
  session?: string;
  sourceRange?: SourceRange;
  slideIndex: number;
  visualization?: VisualizationConfig;
  visualizationDiagnostics?: VisualizationDiagnostic[];
}

export interface CourseSlide {
  id: string;
  index: number;
  title: string;
  source: string;
  html: string;
  codeCells: CodeCell[];
}

export interface CourseDocument {
  title: string;
  source: string;
  slides: CourseSlide[];
  codeCells: CodeCell[];
}
