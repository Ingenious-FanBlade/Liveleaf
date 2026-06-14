use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ::markdown::mdast::Node;
use ::markdown::{to_mdast, ParseOptions};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const FORMAT_NAME: &str = "stream-learn-project";
const SCHEMA_VERSION: u32 = 1;
const DOCUMENT_PATH: &str = "document.sl.md";

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProjectPackageRequest {
    pub source: String,
    pub output_path: String,
    pub source_file_path: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveProjectPackageResponse {
    pub output_path: String,
    pub title: String,
    pub bundled_asset_count: usize,
    pub missing_asset_count: usize,
    pub external_asset_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectPackageRequest {
    pub package_path: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectPackageResponse {
    pub package_path: String,
    pub title: String,
    pub source: String,
    pub source_file_path: String,
    pub working_directory: String,
    pub assets: Vec<OpenProjectPackageAsset>,
    pub bundled_asset_count: usize,
    pub missing_asset_count: usize,
    pub external_asset_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectPackageAsset {
    pub id: String,
    pub kind: String,
    pub source: String,
    pub package_path: Option<String>,
    pub file_path: Option<String>,
    pub mime: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ProjectManifest {
    schema_version: u32,
    format: String,
    document_path: String,
    title: String,
    assets: Vec<ManifestAsset>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ManifestAsset {
    id: String,
    kind: PackageAssetKind,
    source: String,
    package_path: Option<String>,
    mime: Option<String>,
    sha256: Option<String>,
    bytes: Option<u64>,
    status: PackageAssetStatus,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum PackageAssetKind {
    Image,
    Video,
    Audio,
    Other,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum PackageAssetStatus {
    Bundled,
    Missing,
    External,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResourceReference {
    kind: PackageAssetKind,
    source: String,
}

struct BundledAssetFile {
    source_path: PathBuf,
    package_path: String,
}

#[tauri::command]
pub async fn save_project_package(
    request: SaveProjectPackageRequest,
) -> Result<SaveProjectPackageResponse, String> {
    tauri::async_runtime::spawn_blocking(move || save_project_package_blocking(request))
        .await
        .map_err(|error| format!("Project package task failed: {error}"))?
}

#[tauri::command]
pub async fn open_project_package(
    request: OpenProjectPackageRequest,
) -> Result<OpenProjectPackageResponse, String> {
    tauri::async_runtime::spawn_blocking(move || open_project_package_blocking(request))
        .await
        .map_err(|error| format!("Project package open task failed: {error}"))?
}

fn save_project_package_blocking(
    request: SaveProjectPackageRequest,
) -> Result<SaveProjectPackageResponse, String> {
    let output_path = PathBuf::from(request.output_path.trim());

    if output_path.as_os_str().is_empty() {
        return Err("Choose a project package output path.".to_string());
    }

    let output_parent = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            "Project package output path must include a parent directory.".to_string()
        })?;
    fs::create_dir_all(output_parent)
        .map_err(|error| format!("Failed to create output directory: {error}"))?;

    let base_dir = resource_base_dir(&request, &output_path)?;
    let title = crate::markdown::parse_course(&request.source)
        .map(|document| document.title)
        .unwrap_or_else(|_| "Untitled Lesson".to_string());
    let references = collect_resource_references(&request.source);
    let (manifest_assets, bundled_files, warnings) = build_asset_manifest(&base_dir, references)?;
    let manifest = ProjectManifest {
        schema_version: SCHEMA_VERSION,
        format: FORMAT_NAME.to_string(),
        document_path: DOCUMENT_PATH.to_string(),
        title: title.clone(),
        assets: manifest_assets,
        warnings,
    };

    write_package(&output_path, &request.source, &manifest, &bundled_files)?;

    let response = SaveProjectPackageResponse {
        output_path: output_path.to_string_lossy().to_string(),
        title,
        bundled_asset_count: manifest
            .assets
            .iter()
            .filter(|asset| asset.status == PackageAssetStatus::Bundled)
            .count(),
        missing_asset_count: manifest
            .assets
            .iter()
            .filter(|asset| asset.status == PackageAssetStatus::Missing)
            .count(),
        external_asset_count: manifest
            .assets
            .iter()
            .filter(|asset| asset.status == PackageAssetStatus::External)
            .count(),
        warnings: manifest.warnings.clone(),
    };

    Ok(response)
}

fn open_project_package_blocking(
    request: OpenProjectPackageRequest,
) -> Result<OpenProjectPackageResponse, String> {
    let package_path = PathBuf::from(request.package_path.trim());

    if package_path.as_os_str().is_empty() {
        return Err("Choose a project package to open.".to_string());
    }

    let package_path = package_path
        .canonicalize()
        .map_err(|error| format!("Failed to resolve project package path: {error}"))?;
    let file = File::open(&package_path)
        .map_err(|error| format!("Failed to open project package: {error}"))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| format!("Project package is not a ZIP file: {error}"))?;
    let manifest = read_manifest_from_archive(&mut archive)?;

    validate_manifest(&manifest)?;

    let source = read_archive_text(&mut archive, &manifest.document_path)?;
    let working_directory = create_open_package_directory(&package_path)?;
    let source_file_path = working_directory.join(DOCUMENT_PATH);

    write_extracted_file(&source_file_path, source.as_bytes())?;

    let mut warnings = manifest.warnings.clone();
    let mut opened_assets = Vec::new();

    for asset in &manifest.assets {
        let mut file_path = None;
        let mut status = asset.status;

        if asset.status == PackageAssetStatus::Bundled {
            match extract_bundled_asset(&mut archive, &working_directory, asset) {
                Ok(Some(extracted_path)) => {
                    restore_original_asset_path(
                        &working_directory,
                        asset,
                        &extracted_path,
                        &mut warnings,
                    );
                    file_path = Some(extracted_path.to_string_lossy().to_string());
                }
                Ok(None) => {
                    status = PackageAssetStatus::Missing;
                    warnings.push(format!("Packaged resource is missing: {}", asset.source));
                }
                Err(error) => {
                    status = PackageAssetStatus::Blocked;
                    warnings.push(error);
                }
            }
        }

        opened_assets.push(OpenProjectPackageAsset {
            id: asset.id.clone(),
            kind: kind_label(asset.kind).to_string(),
            source: asset.source.clone(),
            package_path: asset.package_path.clone(),
            file_path,
            mime: asset.mime.clone(),
            status: status_label(status).to_string(),
        });
    }

    Ok(OpenProjectPackageResponse {
        package_path: package_path.to_string_lossy().to_string(),
        title: manifest.title,
        source,
        source_file_path: source_file_path.to_string_lossy().to_string(),
        working_directory: working_directory.to_string_lossy().to_string(),
        bundled_asset_count: opened_assets
            .iter()
            .filter(|asset| asset.status == "bundled")
            .count(),
        missing_asset_count: opened_assets
            .iter()
            .filter(|asset| asset.status == "missing")
            .count(),
        external_asset_count: opened_assets
            .iter()
            .filter(|asset| asset.status == "external")
            .count(),
        assets: opened_assets,
        warnings,
    })
}

fn resource_base_dir(
    request: &SaveProjectPackageRequest,
    output_path: &Path,
) -> Result<PathBuf, String> {
    if let Some(source_file_path) = request
        .source_file_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        let source_path = Path::new(source_file_path);

        if let Some(parent) = source_path.parent() {
            return canonicalize_existing_dir(parent);
        }
    }

    let parent = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            "Project package output path must include a parent directory.".to_string()
        })?;

    canonicalize_existing_dir(parent)
}

fn canonicalize_existing_dir(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize().map_err(|error| {
        format!(
            "Failed to resolve resource directory '{}': {error}",
            path.display()
        )
    })
}

fn read_manifest_from_archive(archive: &mut ZipArchive<File>) -> Result<ProjectManifest, String> {
    let manifest_json = read_archive_text(archive, "manifest.json")?;

    serde_json::from_str(&manifest_json)
        .map_err(|error| format!("Project manifest is not valid JSON: {error}"))
}

fn validate_manifest(manifest: &ProjectManifest) -> Result<(), String> {
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "Unsupported project package schema version: {}",
            manifest.schema_version
        ));
    }

    if manifest.format != FORMAT_NAME {
        return Err(format!(
            "Unsupported project package format: {}",
            manifest.format
        ));
    }

    if !is_safe_package_entry_path(Path::new(&manifest.document_path)) {
        return Err("Project document path is not safe.".to_string());
    }

    Ok(())
}

fn read_archive_text(archive: &mut ZipArchive<File>, entry_path: &str) -> Result<String, String> {
    let mut entry = archive
        .by_name(entry_path)
        .map_err(|error| format!("Missing package entry '{entry_path}': {error}"))?;
    let mut value = String::new();

    entry
        .read_to_string(&mut value)
        .map_err(|error| format!("Failed to read package entry '{entry_path}': {error}"))?;

    Ok(value)
}

fn create_open_package_directory(package_path: &Path) -> Result<PathBuf, String> {
    let mut directory = std::env::temp_dir();
    let package_name = package_path
        .file_stem()
        .and_then(|name| name.to_str())
        .map(sanitize_file_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "project".to_string());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    directory.push(format!(
        "stream-learn-open-{package_name}-{}-{timestamp}",
        std::process::id()
    ));
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Failed to create temporary project directory: {error}"))?;

    Ok(directory)
}

fn extract_bundled_asset(
    archive: &mut ZipArchive<File>,
    working_directory: &Path,
    asset: &ManifestAsset,
) -> Result<Option<PathBuf>, String> {
    let package_path = match asset.package_path.as_deref() {
        Some(path) if !path.trim().is_empty() => path,
        _ => return Ok(None),
    };
    let package_entry_path = Path::new(package_path);

    if !is_safe_package_entry_path(package_entry_path) {
        return Err(format!(
            "Skipped unsafe packaged resource path for {}: {}",
            asset.source, package_path
        ));
    }

    let output_path = working_directory.join(package_entry_path);
    let mut entry = match archive.by_name(package_path) {
        Ok(entry) => entry,
        Err(_) => return Ok(None),
    };

    if entry.is_dir() {
        return Ok(None);
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create asset directory: {error}"))?;
    }

    let mut output_file = File::create(&output_path)
        .map_err(|error| format!("Failed to extract resource '{}': {error}", asset.source))?;
    io::copy(&mut entry, &mut output_file)
        .map_err(|error| format!("Failed to extract resource '{}': {error}", asset.source))?;

    Ok(Some(output_path))
}

fn restore_original_asset_path(
    working_directory: &Path,
    asset: &ManifestAsset,
    extracted_path: &Path,
    warnings: &mut Vec<String>,
) {
    let local_path_part = local_file_part(&asset.source);
    let original_path = Path::new(local_path_part);

    if !is_safe_relative_path(original_path) {
        return;
    }

    let target_path = working_directory.join(original_path);

    if target_path == extracted_path || target_path.exists() {
        return;
    }

    if let Some(parent) = target_path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            warnings.push(format!(
                "Could not restore original resource path '{}': {error}",
                asset.source
            ));
            return;
        }
    }

    if let Err(error) = fs::copy(extracted_path, &target_path) {
        warnings.push(format!(
            "Could not restore original resource path '{}': {error}",
            asset.source
        ));
    }
}

fn write_extracted_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create extracted file directory: {error}"))?;
    }

    fs::write(path, bytes).map_err(|error| {
        format!(
            "Failed to write extracted file '{}': {error}",
            path.display()
        )
    })
}

fn is_safe_package_entry_path(path: &Path) -> bool {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return false;
    }

    path.components().all(|component| match component {
        Component::Normal(_) | Component::CurDir => true,
        Component::ParentDir | Component::RootDir | Component::Prefix(_) => false,
    })
}

fn kind_label(kind: PackageAssetKind) -> &'static str {
    match kind {
        PackageAssetKind::Image => "image",
        PackageAssetKind::Video => "video",
        PackageAssetKind::Audio => "audio",
        PackageAssetKind::Other => "other",
    }
}

fn status_label(status: PackageAssetStatus) -> &'static str {
    match status {
        PackageAssetStatus::Bundled => "bundled",
        PackageAssetStatus::Missing => "missing",
        PackageAssetStatus::External => "external",
        PackageAssetStatus::Blocked => "blocked",
    }
}

fn collect_resource_references(source: &str) -> Vec<ResourceReference> {
    let mut references = Vec::new();

    if let Ok(ast) = to_mdast(source, &ParseOptions::gfm()) {
        collect_markdown_image_references(&ast, &mut references);
    }

    collect_media_directive_references(source, &mut references);

    references
}

fn collect_markdown_image_references(node: &Node, references: &mut Vec<ResourceReference>) {
    if let Node::Image(image) = node {
        references.push(ResourceReference {
            kind: PackageAssetKind::Image,
            source: image.url.clone(),
        });
    }

    if let Some(children) = node.children() {
        for child in children {
            collect_markdown_image_references(child, references);
        }
    }
}

fn collect_media_directive_references(source: &str, references: &mut Vec<ResourceReference>) {
    for line in source.lines() {
        if let Some(media) = parse_media_directive(line) {
            references.push(ResourceReference {
                kind: media.kind,
                source: media.source,
            });

            if media.kind == PackageAssetKind::Video {
                if let Some(poster) = media.params.attrs.get("poster") {
                    references.push(ResourceReference {
                        kind: PackageAssetKind::Image,
                        source: poster.clone(),
                    });
                }
            }
        }
    }
}

struct ParsedMediaDirective {
    kind: PackageAssetKind,
    source: String,
    params: ParsedParams,
}

#[derive(Debug, Default)]
struct ParsedParams {
    attrs: HashMap<String, String>,
}

fn parse_media_directive(line: &str) -> Option<ParsedMediaDirective> {
    let trimmed = line.trim();
    let (kind, after_prefix) = if let Some(rest) = trimmed.strip_prefix("::video[") {
        (PackageAssetKind::Video, rest)
    } else if let Some(rest) = trimmed.strip_prefix("::audio[") {
        (PackageAssetKind::Audio, rest)
    } else {
        return None;
    };

    let label_end = after_prefix.find("](")?;
    let after_label = &after_prefix[label_end + 2..];
    let source_end = after_label.find(')')?;
    let source = after_label[..source_end].trim().to_string();
    let suffix = after_label[source_end + 1..].trim();
    let params = if suffix.starts_with('{') && suffix.ends_with('}') {
        parse_params(&suffix[1..suffix.len() - 1])
    } else if suffix.is_empty() {
        ParsedParams::default()
    } else {
        return None;
    };

    Some(ParsedMediaDirective {
        kind,
        source,
        params,
    })
}

fn parse_params(params: &str) -> ParsedParams {
    let mut parsed = ParsedParams::default();
    let mut chars = params.chars().peekable();

    loop {
        skip_whitespace(&mut chars);

        if chars.peek().is_none() {
            break;
        }

        let key = read_until(&mut chars, |ch| ch.is_whitespace() || ch == '=');

        if key.is_empty() {
            chars.next();
            continue;
        }

        if chars.peek() == Some(&'=') {
            chars.next();
            parsed.attrs.insert(key, read_param_value(&mut chars));
        } else {
            let _ = key;
        }
    }

    parsed
}

fn read_param_value<I>(chars: &mut std::iter::Peekable<I>) -> String
where
    I: Iterator<Item = char>,
{
    match chars.peek().copied() {
        Some('"') | Some('\'') => {
            let quote = chars.next().expect("peeked quote should exist");
            let value = read_until(chars, |ch| ch == quote);

            if chars.peek() == Some(&quote) {
                chars.next();
            }

            value
        }
        Some(_) => read_until(chars, |ch| ch.is_whitespace()),
        None => String::new(),
    }
}

fn read_until<I, F>(chars: &mut std::iter::Peekable<I>, should_stop: F) -> String
where
    I: Iterator<Item = char>,
    F: Fn(char) -> bool,
{
    let mut value = String::new();

    while let Some(ch) = chars.peek().copied() {
        if should_stop(ch) {
            break;
        }

        value.push(ch);
        chars.next();
    }

    value
}

fn skip_whitespace<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while chars.peek().is_some_and(|ch| ch.is_whitespace()) {
        chars.next();
    }
}

fn build_asset_manifest(
    base_dir: &Path,
    references: Vec<ResourceReference>,
) -> Result<(Vec<ManifestAsset>, Vec<BundledAssetFile>, Vec<String>), String> {
    let mut assets = Vec::new();
    let mut bundled_files = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_external_sources = HashSet::new();
    let mut seen_blocked_sources = HashSet::new();
    let mut seen_missing_sources = HashSet::new();
    let mut seen_canonical_paths = HashSet::new();
    let mut used_package_paths = HashSet::new();

    for reference in references {
        if reference.source.trim().is_empty() {
            let warning = "Skipping an empty resource reference.".to_string();
            warnings.push(warning);
            continue;
        }

        if is_external_source(&reference.source) {
            if !seen_external_sources.insert(reference.source.clone()) {
                continue;
            }

            assets.push(ManifestAsset {
                id: next_asset_id(assets.len()),
                kind: reference.kind,
                source: reference.source,
                package_path: None,
                mime: None,
                sha256: None,
                bytes: None,
                status: PackageAssetStatus::External,
            });
            continue;
        }

        let local_path_part = local_file_part(&reference.source).to_string();
        let relative_path = Path::new(&local_path_part);

        if !is_safe_relative_path(relative_path) {
            if seen_blocked_sources.insert(reference.source.clone()) {
                warnings.push(format!(
                    "Skipped resource outside the project directory: {}",
                    reference.source
                ));
                assets.push(ManifestAsset {
                    id: next_asset_id(assets.len()),
                    kind: reference.kind,
                    source: reference.source,
                    package_path: None,
                    mime: None,
                    sha256: None,
                    bytes: None,
                    status: PackageAssetStatus::Blocked,
                });
            }

            continue;
        }

        let candidate_path = base_dir.join(relative_path);
        let canonical_path = match candidate_path.canonicalize() {
            Ok(path) => path,
            Err(_) => {
                if seen_missing_sources.insert(reference.source.clone()) {
                    warnings.push(format!("Missing resource: {}", reference.source));
                    assets.push(ManifestAsset {
                        id: next_asset_id(assets.len()),
                        kind: reference.kind,
                        source: reference.source,
                        package_path: None,
                        mime: mime_for_path(relative_path),
                        sha256: None,
                        bytes: None,
                        status: PackageAssetStatus::Missing,
                    });
                }

                continue;
            }
        };

        if !canonical_path.starts_with(base_dir) {
            if seen_blocked_sources.insert(reference.source.clone()) {
                warnings.push(format!(
                    "Skipped resource outside the project directory: {}",
                    reference.source
                ));
                assets.push(ManifestAsset {
                    id: next_asset_id(assets.len()),
                    kind: reference.kind,
                    source: reference.source,
                    package_path: None,
                    mime: mime_for_path(&canonical_path),
                    sha256: None,
                    bytes: None,
                    status: PackageAssetStatus::Blocked,
                });
            }

            continue;
        }

        if !canonical_path.is_file() {
            if seen_missing_sources.insert(reference.source.clone()) {
                warnings.push(format!("Resource is not a file: {}", reference.source));
                assets.push(ManifestAsset {
                    id: next_asset_id(assets.len()),
                    kind: reference.kind,
                    source: reference.source,
                    package_path: None,
                    mime: mime_for_path(&canonical_path),
                    sha256: None,
                    bytes: None,
                    status: PackageAssetStatus::Missing,
                });
            }

            continue;
        }

        if !seen_canonical_paths.insert(canonical_path.clone()) {
            continue;
        }

        let (sha256, bytes) = hash_file(&canonical_path)
            .map_err(|error| format!("Failed to hash '{}': {error}", canonical_path.display()))?;
        let package_path = unique_package_path(
            reference.kind,
            &canonical_path,
            &sha256,
            &mut used_package_paths,
        );

        bundled_files.push(BundledAssetFile {
            source_path: canonical_path.clone(),
            package_path: package_path.clone(),
        });
        assets.push(ManifestAsset {
            id: next_asset_id(assets.len()),
            kind: reference.kind,
            source: reference.source,
            package_path: Some(package_path),
            mime: mime_for_path(&canonical_path),
            sha256: Some(sha256),
            bytes: Some(bytes),
            status: PackageAssetStatus::Bundled,
        });
    }

    Ok((assets, bundled_files, warnings))
}

fn next_asset_id(index: usize) -> String {
    format!("asset-{}", index + 1)
}

fn is_external_source(source: &str) -> bool {
    let lower = source.trim().to_ascii_lowercase();

    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("//")
        || lower.starts_with("data:")
        || lower.starts_with("mailto:")
}

fn local_file_part(source: &str) -> &str {
    source.split(['#', '?']).next().unwrap_or(source).trim()
}

fn is_safe_relative_path(path: &Path) -> bool {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return false;
    }

    path.components().all(|component| match component {
        Component::Normal(_) | Component::CurDir => true,
        Component::ParentDir | Component::RootDir | Component::Prefix(_) => false,
    })
}

fn hash_file(path: &Path) -> io::Result<(String, u64)> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut total_bytes = 0;
    let mut buffer = [0; 32 * 1024];

    loop {
        let bytes_read = file.read(&mut buffer)?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
        total_bytes += bytes_read as u64;
    }

    Ok((format!("{:x}", hasher.finalize()), total_bytes))
}

fn unique_package_path(
    kind: PackageAssetKind,
    source_path: &Path,
    sha256: &str,
    used_package_paths: &mut HashSet<String>,
) -> String {
    let directory = match kind {
        PackageAssetKind::Image => "image",
        PackageAssetKind::Video => "video",
        PackageAssetKind::Audio => "audio",
        PackageAssetKind::Other => "other",
    };
    let file_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(sanitize_file_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "asset".to_string());
    let hash_prefix = &sha256[..8.min(sha256.len())];
    let base_path = format!("assets/{directory}/{hash_prefix}-{file_name}");

    if used_package_paths.insert(base_path.clone()) {
        return base_path;
    }

    for counter in 2.. {
        let candidate = format!("assets/{directory}/{hash_prefix}-{counter}-{file_name}");

        if used_package_paths.insert(candidate.clone()) {
            return candidate;
        }
    }

    unreachable!("counter loop always returns")
}

fn sanitize_file_name(file_name: &str) -> String {
    file_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn mime_for_path(path: &Path) -> Option<String> {
    mime_guess::from_path(path)
        .first()
        .map(|mime| mime.essence_str().to_string())
}

fn write_package(
    output_path: &Path,
    source: &str,
    manifest: &ProjectManifest,
    bundled_files: &[BundledAssetFile],
) -> Result<(), String> {
    let file = File::create(output_path)
        .map_err(|error| format!("Failed to create project package: {error}"))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let manifest_json = serde_json::to_vec_pretty(manifest)
        .map_err(|error| format!("Failed to serialize project manifest: {error}"))?;

    zip.start_file("manifest.json", options)
        .map_err(|error| format!("Failed to write manifest entry: {error}"))?;
    zip.write_all(&manifest_json)
        .map_err(|error| format!("Failed to write manifest: {error}"))?;

    zip.start_file(DOCUMENT_PATH, options)
        .map_err(|error| format!("Failed to write document entry: {error}"))?;
    zip.write_all(source.as_bytes())
        .map_err(|error| format!("Failed to write Markdown document: {error}"))?;

    for asset in bundled_files {
        zip.start_file(&asset.package_path, options)
            .map_err(|error| {
                format!(
                    "Failed to write asset entry '{}': {error}",
                    asset.package_path
                )
            })?;
        let mut file = File::open(&asset.source_path).map_err(|error| {
            format!(
                "Failed to open asset '{}': {error}",
                asset.source_path.display()
            )
        })?;
        io::copy(&mut file, &mut zip).map_err(|error| {
            format!(
                "Failed to copy asset '{}': {error}",
                asset.source_path.display()
            )
        })?;
    }

    zip.finish()
        .map_err(|error| format!("Failed to finish project package: {error}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use zip::ZipArchive;

    #[test]
    fn saves_markdown_only_package() {
        let temp_dir = temp_test_dir("markdown-only");
        let output_path = temp_dir.join("lesson.slpkg");
        let source = "# Lesson\n\nHello package.\n";
        let response = save_project_package_blocking(SaveProjectPackageRequest {
            source: source.to_string(),
            output_path: output_path.to_string_lossy().to_string(),
            source_file_path: None,
        })
        .expect("package should save");

        assert_eq!(response.bundled_asset_count, 0);
        assert_eq!(response.missing_asset_count, 0);
        assert_eq!(response.external_asset_count, 0);

        let (manifest, document) = read_manifest_and_document(&output_path);
        assert_eq!(manifest.schema_version, SCHEMA_VERSION);
        assert_eq!(manifest.format, FORMAT_NAME);
        assert_eq!(manifest.document_path, DOCUMENT_PATH);
        assert_eq!(manifest.title, "Lesson");
        assert!(manifest.assets.is_empty());
        assert_eq!(document, source);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn bundles_images_media_audio_and_video_poster() {
        let temp_dir = temp_test_dir("assets");
        let asset_dir = temp_dir.join("assets");
        fs::create_dir_all(&asset_dir).expect("asset dir");
        fs::write(asset_dir.join("diagram.png"), b"image bytes").expect("image");
        fs::write(asset_dir.join("demo.mp4"), b"video bytes").expect("video");
        fs::write(asset_dir.join("poster.jpg"), b"poster bytes").expect("poster");
        fs::write(asset_dir.join("intro.mp3"), b"audio bytes").expect("audio");

        let source = "# Media\n\n![Diagram](./assets/diagram.png)\n\n::video[Demo](./assets/demo.mp4){controls poster=\"./assets/poster.jpg\"}\n\n::audio[Intro](./assets/intro.mp3){controls}\n";
        let output_path = temp_dir.join("lesson.slpkg");
        let response = save_project_package_blocking(SaveProjectPackageRequest {
            source: source.to_string(),
            output_path: output_path.to_string_lossy().to_string(),
            source_file_path: Some(temp_dir.join("lesson.sl.md").to_string_lossy().to_string()),
        })
        .expect("package should save");

        assert_eq!(response.bundled_asset_count, 4);
        assert!(response.warnings.is_empty());

        let (manifest, document) = read_manifest_and_document(&output_path);
        assert_eq!(document, source);
        assert_eq!(manifest.assets.len(), 4);
        assert_eq!(
            manifest
                .assets
                .iter()
                .filter(|asset| asset.kind == PackageAssetKind::Image)
                .count(),
            2
        );
        assert!(manifest.assets.iter().any(|asset| {
            asset.kind == PackageAssetKind::Video
                && asset
                    .package_path
                    .as_deref()
                    .is_some_and(|path| path.starts_with("assets/video/"))
        }));
        assert!(manifest.assets.iter().any(|asset| {
            asset.kind == PackageAssetKind::Audio
                && asset
                    .package_path
                    .as_deref()
                    .is_some_and(|path| path.starts_with("assets/audio/"))
        }));

        let zip_names = read_zip_names(&output_path);
        assert!(zip_names.contains(&"manifest.json".to_string()));
        assert!(zip_names.contains(&DOCUMENT_PATH.to_string()));
        assert_eq!(
            zip_names
                .iter()
                .filter(|name| name.starts_with("assets/"))
                .count(),
            4
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn missing_and_external_assets_do_not_block_save() {
        let temp_dir = temp_test_dir("missing-external");
        let output_path = temp_dir.join("lesson.slpkg");
        let source = "# Assets\n\n![Missing](./assets/missing.png)\n\n![Remote](https://example.com/remote.png)\n";
        let response = save_project_package_blocking(SaveProjectPackageRequest {
            source: source.to_string(),
            output_path: output_path.to_string_lossy().to_string(),
            source_file_path: None,
        })
        .expect("package should save with warnings");

        assert_eq!(response.bundled_asset_count, 0);
        assert_eq!(response.missing_asset_count, 1);
        assert_eq!(response.external_asset_count, 1);
        assert_eq!(response.warnings.len(), 1);

        let (manifest, _) = read_manifest_and_document(&output_path);
        assert!(manifest
            .assets
            .iter()
            .any(|asset| asset.status == PackageAssetStatus::Missing));
        assert!(manifest
            .assets
            .iter()
            .any(|asset| asset.status == PackageAssetStatus::External));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn parent_directory_assets_are_blocked() {
        let temp_dir = temp_test_dir("blocked");
        let output_path = temp_dir.join("lesson.slpkg");
        let source = "# Blocked\n\n![Secret](../secret.png)\n";
        let response = save_project_package_blocking(SaveProjectPackageRequest {
            source: source.to_string(),
            output_path: output_path.to_string_lossy().to_string(),
            source_file_path: None,
        })
        .expect("package should save with blocked warning");

        assert_eq!(response.bundled_asset_count, 0);
        assert_eq!(response.warnings.len(), 1);

        let (manifest, _) = read_manifest_and_document(&output_path);
        assert_eq!(manifest.assets.len(), 1);
        assert_eq!(manifest.assets[0].status, PackageAssetStatus::Blocked);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn duplicate_asset_references_are_bundled_once() {
        let temp_dir = temp_test_dir("duplicate");
        let asset_dir = temp_dir.join("assets");
        fs::create_dir_all(&asset_dir).expect("asset dir");
        fs::write(asset_dir.join("diagram.png"), b"image bytes").expect("image");

        let output_path = temp_dir.join("lesson.slpkg");
        let source =
            "# Duplicate\n\n![One](./assets/diagram.png)\n\n![Two](./assets/diagram.png)\n";
        let response = save_project_package_blocking(SaveProjectPackageRequest {
            source: source.to_string(),
            output_path: output_path.to_string_lossy().to_string(),
            source_file_path: None,
        })
        .expect("package should save");

        assert_eq!(response.bundled_asset_count, 1);

        let (manifest, _) = read_manifest_and_document(&output_path);
        assert_eq!(manifest.assets.len(), 1);
        assert_eq!(
            read_zip_names(&output_path)
                .iter()
                .filter(|name| name.starts_with("assets/"))
                .count(),
            1
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn opens_saved_package_and_extracts_assets() {
        let temp_dir = temp_test_dir("open-saved");
        let asset_dir = temp_dir.join("assets");
        fs::create_dir_all(&asset_dir).expect("asset dir");
        fs::write(asset_dir.join("diagram.png"), b"image bytes").expect("image");

        let output_path = temp_dir.join("lesson.slpkg");
        let source = "# Open Me\n\n![Diagram](./assets/diagram.png)\n";
        save_project_package_blocking(SaveProjectPackageRequest {
            source: source.to_string(),
            output_path: output_path.to_string_lossy().to_string(),
            source_file_path: Some(temp_dir.join("lesson.sl.md").to_string_lossy().to_string()),
        })
        .expect("package should save");

        let response = open_project_package_blocking(OpenProjectPackageRequest {
            package_path: output_path.to_string_lossy().to_string(),
        })
        .expect("package should open");

        assert_eq!(response.title, "Open Me");
        assert_eq!(response.source, source);
        assert_eq!(response.bundled_asset_count, 1);
        assert_eq!(response.assets.len(), 1);

        let extracted_asset = response.assets[0]
            .file_path
            .as_deref()
            .expect("asset should have extracted path");
        assert!(Path::new(extracted_asset).is_file());
        assert!(Path::new(&response.source_file_path).is_file());
        assert!(Path::new(&response.working_directory)
            .join("assets")
            .join("diagram.png")
            .is_file());

        let _ = fs::remove_dir_all(temp_dir);
        let _ = fs::remove_dir_all(response.working_directory);
    }

    #[test]
    fn opening_package_reports_missing_bundled_entries() {
        let temp_dir = temp_test_dir("open-missing-entry");
        let output_path = temp_dir.join("broken.slpkg");
        let manifest = ProjectManifest {
            schema_version: SCHEMA_VERSION,
            format: FORMAT_NAME.to_string(),
            document_path: DOCUMENT_PATH.to_string(),
            title: "Broken".to_string(),
            assets: vec![ManifestAsset {
                id: "asset-1".to_string(),
                kind: PackageAssetKind::Image,
                source: "./assets/missing.png".to_string(),
                package_path: Some("assets/image/missing.png".to_string()),
                mime: Some("image/png".to_string()),
                sha256: None,
                bytes: None,
                status: PackageAssetStatus::Bundled,
            }],
            warnings: Vec::new(),
        };

        write_test_package(&output_path, "# Broken\n", &manifest, &[]);

        let response = open_project_package_blocking(OpenProjectPackageRequest {
            package_path: output_path.to_string_lossy().to_string(),
        })
        .expect("package should open with warnings");

        assert_eq!(response.bundled_asset_count, 0);
        assert_eq!(response.missing_asset_count, 1);
        assert_eq!(response.warnings.len(), 1);

        let _ = fs::remove_dir_all(temp_dir);
        let _ = fs::remove_dir_all(response.working_directory);
    }

    #[test]
    fn opening_package_blocks_unsafe_asset_entries() {
        let temp_dir = temp_test_dir("open-blocked-entry");
        let output_path = temp_dir.join("blocked.slpkg");
        let manifest = ProjectManifest {
            schema_version: SCHEMA_VERSION,
            format: FORMAT_NAME.to_string(),
            document_path: DOCUMENT_PATH.to_string(),
            title: "Blocked".to_string(),
            assets: vec![ManifestAsset {
                id: "asset-1".to_string(),
                kind: PackageAssetKind::Image,
                source: "./assets/image.png".to_string(),
                package_path: Some("../image.png".to_string()),
                mime: Some("image/png".to_string()),
                sha256: None,
                bytes: None,
                status: PackageAssetStatus::Bundled,
            }],
            warnings: Vec::new(),
        };

        write_test_package(&output_path, "# Blocked\n", &manifest, &[]);

        let response = open_project_package_blocking(OpenProjectPackageRequest {
            package_path: output_path.to_string_lossy().to_string(),
        })
        .expect("package should open with blocked warning");

        assert_eq!(response.bundled_asset_count, 0);
        assert_eq!(response.assets[0].status, "blocked");
        assert_eq!(response.warnings.len(), 1);

        let _ = fs::remove_dir_all(temp_dir);
        let _ = fs::remove_dir_all(response.working_directory);
    }

    fn temp_test_dir(label: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();

        path.push(format!(
            "stream-learn-project-package-{label}-{}-{timestamp}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp test dir");

        path
    }

    fn read_manifest_and_document(output_path: &Path) -> (ProjectManifest, String) {
        let file = File::open(output_path).expect("package file");
        let mut archive = ZipArchive::new(file).expect("zip archive");
        let manifest = {
            let mut manifest_entry = archive.by_name("manifest.json").expect("manifest entry");
            let mut manifest_json = String::new();
            manifest_entry
                .read_to_string(&mut manifest_json)
                .expect("manifest contents");
            serde_json::from_str::<ProjectManifest>(&manifest_json).expect("manifest json")
        };
        let document = {
            let mut document_entry = archive.by_name(DOCUMENT_PATH).expect("document entry");
            let mut document = String::new();
            document_entry
                .read_to_string(&mut document)
                .expect("document contents");
            document
        };

        (manifest, document)
    }

    fn read_zip_names(output_path: &Path) -> Vec<String> {
        let file = File::open(output_path).expect("package file");
        let mut archive = ZipArchive::new(file).expect("zip archive");
        let mut names = Vec::new();

        for index in 0..archive.len() {
            let file = archive.by_index(index).expect("zip entry");
            names.push(file.name().to_string());
        }

        names
    }

    fn write_test_package(
        output_path: &Path,
        source: &str,
        manifest: &ProjectManifest,
        entries: &[(&str, &[u8])],
    ) {
        let file = File::create(output_path).expect("test package");
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let manifest_json = serde_json::to_vec_pretty(manifest).expect("manifest json");

        zip.start_file("manifest.json", options)
            .expect("manifest entry");
        zip.write_all(&manifest_json).expect("manifest write");
        zip.start_file(DOCUMENT_PATH, options)
            .expect("document entry");
        zip.write_all(source.as_bytes()).expect("document write");

        for (path, bytes) in entries {
            zip.start_file(*path, options).expect("asset entry");
            zip.write_all(bytes).expect("asset write");
        }

        zip.finish().expect("finish package");
    }
}
