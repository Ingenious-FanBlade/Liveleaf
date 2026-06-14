import { invoke } from "@tauri-apps/api/core";

export interface SaveProjectPackageRequest {
  source: string;
  outputPath: string;
  sourceFilePath?: string;
}

export interface SaveProjectPackageResponse {
  outputPath: string;
  title: string;
  bundledAssetCount: number;
  missingAssetCount: number;
  externalAssetCount: number;
  warnings: string[];
}

export interface OpenProjectPackageRequest {
  packagePath: string;
}

export interface OpenProjectPackageAsset {
  id: string;
  kind: "image" | "video" | "audio" | "other";
  source: string;
  packagePath?: string;
  filePath?: string;
  mime?: string;
  status: "bundled" | "missing" | "external" | "blocked";
}

export interface OpenProjectPackageResponse {
  packagePath: string;
  title: string;
  source: string;
  sourceFilePath: string;
  workingDirectory: string;
  assets: OpenProjectPackageAsset[];
  bundledAssetCount: number;
  missingAssetCount: number;
  externalAssetCount: number;
  warnings: string[];
}

export function saveProjectPackage(
  request: SaveProjectPackageRequest,
): Promise<SaveProjectPackageResponse> {
  return invoke<SaveProjectPackageResponse>("save_project_package", { request });
}

export function openProjectPackage(
  request: OpenProjectPackageRequest,
): Promise<OpenProjectPackageResponse> {
  return invoke<OpenProjectPackageResponse>("open_project_package", { request });
}
