use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "liveleaf-cli-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn source(&self, source: &str) {
        fs::write(self.0.join("lesson.ll.md"), source).unwrap();
    }

    fn invoke(&self, args: &[&str], stdin: Option<&str>) -> (i32, Value) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_liveleaf-cli"))
            .current_dir(&self.0)
            .arg("--json")
            .args(args)
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(source) = stdin {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(source.as_bytes())
                .unwrap();
        }
        decode(child.wait_with_output().unwrap())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn decode(output: Output) -> (i32, Value) {
    assert!(
        output.stderr.is_empty(),
        "JSON mode must keep stderr clean: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "Expected exactly one JSON document: {e}; stdout={}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    assert_eq!(value["schemaVersion"], 1);
    assert!(value["diagnostics"].is_array());
    (output.status.code().unwrap(), value)
}

fn has_diagnostic(value: &Value, code: &str) -> bool {
    value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["code"] == code)
}

fn python_available() -> bool {
    let names = if cfg!(windows) {
        vec!["python", "py"]
    } else {
        vec!["python3", "python"]
    };
    names.iter().any(|name| {
        Command::new(name)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    })
}

#[test]
fn spec_describes_current_capabilities_and_its_examples_validate() {
    let fixture = Fixture::new();
    let (code, response) = fixture.invoke(&["spec"], None);
    assert_eq!(code, 0);
    assert_eq!(response["ok"], true);
    assert_eq!(
        response["data"]["code"]["attributes"]["session"]["sharedState"],
        false
    );
    assert_eq!(response["data"]["code"]["sandboxed"], false);
    for (name, example) in response["data"]["examples"].as_object().unwrap() {
        let (code, response) = fixture.invoke(&["validate", "--stdin"], example.as_str());
        assert_eq!(code, 0, "spec example {name}: {response}");
    }
}

#[test]
fn usage_io_help_and_version_all_produce_json() {
    let fixture = Fixture::new();
    for args in [
        vec!["unknown"],
        vec!["run", "lesson.ll.md"],
        vec!["pack", "lesson.ll.md"],
        vec!["spec", "--bad"],
        vec!["spec", "--json=true"],
        vec!["inspect", "--stdin", "lesson.ll.md"],
        vec!["validate", "--stdin", "--cell", "x"],
    ] {
        let (code, value) = fixture.invoke(&args, None);
        assert_eq!(code, 2, "{args:?}");
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "UsageError");
    }
    let (code, value) = fixture.invoke(&["validate", "missing.ll.md"], None);
    assert_eq!(code, 2);
    assert_eq!(value["error"]["code"], "IoError");
    fs::write(fixture.0.join("invalid.ll.md"), [0xff, 0xfe]).unwrap();
    assert_eq!(fixture.invoke(&["inspect", "invalid.ll.md"], None).0, 2);
    for args in [
        vec!["--help"],
        vec!["run", "--help"],
        vec!["--version"],
        vec![],
    ] {
        assert_eq!(fixture.invoke(&args, None).0, 0);
    }
}

#[test]
fn inspect_preserves_unicode_source_ranges_and_never_executes_code() {
    let fixture = Fixture::new();
    let source = "\u{feff}# 第一课\r\n\r\n```python run id=first\r\nopen('executed.txt', 'w').write('bad')\r\n```\r\n\r\n---\r\n\r\n# 第二课\r\n\r\n```python exercise id=second\r\nprint('你好')\r\n```\r\n";
    fs::write(fixture.0.join("中文 lesson.ll.md"), source).unwrap();
    let (code, value) = fixture.invoke(&["inspect", "中文 lesson.ll.md"], None);
    assert_eq!(code, 0);
    assert_eq!(value["data"]["slideCount"], 2);
    assert_eq!(
        value["data"]["document"]["codeCells"][1]["sourceRange"]["startLine"],
        11
    );
    assert_eq!(value["data"]["title"], "第一课");
    assert!(!fixture.0.join("executed.txt").exists());
    let (code, _) = fixture.invoke(&["validate", "中文 lesson.ll.md"], None);
    assert_eq!(code, 0);
    assert!(!fixture.0.join("executed.txt").exists());
}

#[test]
fn validation_reports_authoring_errors_with_locations_and_cell_ids() {
    let fixture = Fixture::new();
    let source = "# Lesson\n\n```python run id=duplicate timeout=nope tests=tests.py\nprint(1)\n```\n\n```python exercise id=duplicate visualize=array watch=invalid.name\nprint(2)\n```\n\n::: column\nText\n";
    let (code, value) = fixture.invoke(&["validate", "--stdin"], Some(source));
    assert_eq!(code, 1);
    for code in [
        "DuplicateCellId",
        "InvalidTimeout",
        "UnknownCodeAttribute",
        "VisualizationConfigError",
        "InvalidColumn",
        "UnclosedDirective",
    ] {
        assert!(has_diagnostic(&value, code), "missing {code}: {value}");
    }
    let duplicate = value["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "DuplicateCellId")
        .unwrap();
    assert_eq!(duplicate["line"], 7);
    assert_eq!(duplicate["cellId"], "duplicate");
    let (code, inspected) = fixture.invoke(&["inspect", "--stdin"], Some(source));
    assert_eq!(code, 1);
    assert_eq!(
        inspected["data"]["document"]["codeCells"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn validation_checks_fences_directives_flags_and_attributes_without_reading_code_as_markup() {
    let fixture = Fixture::new();
    let source = "# Lesson\n\n```text\n::: unknown\n::video[example](missing.mp4){controls}\n```\n\n::: columns\n::: column width=45%\nText\n:::\n::: column width=55%\nText\n:::\n:::\n";
    assert_eq!(fixture.invoke(&["validate", "--stdin"], Some(source)).0, 0);
    for (source, diagnostic) in [
        ("```python run\nprint(1)\n", "UnclosedCodeFence"),
        ("::: notes\nFuture feature\n:::\n", "UnknownDirective"),
        ("```python run exercise\npass\n```", "ConflictingCellKind"),
        ("```python run id=x id=y\npass\n```", "DuplicateAttribute"),
        (
            "```python run id=\"unterminated\npass\n```",
            "UnterminatedAttribute",
        ),
        (
            "```javascript run id=x\nconsole.log(1)\n```",
            "UnsupportedRuntime",
        ),
        ("```python run timeout=0\npass\n```", "InvalidTimeout"),
    ] {
        let (code, value) = fixture.invoke(&["validate", "-"], Some(source));
        assert_eq!(code, 1, "{source}: {value}");
        assert!(has_diagnostic(&value, diagnostic), "{source}: {value}");
    }
}

#[test]
fn resources_use_the_package_policy_and_stdin_base_directory() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.0.join("assets")).unwrap();
    fs::write(fixture.0.join("assets/图片.png"), b"test image").unwrap();
    let source =
        "# Lesson\n\n![image](./assets/图片.png)\n\n![external](https://example.com/image.png)\n";
    let (code, value) = fixture.invoke(
        &[
            "inspect",
            "--stdin",
            "--base-dir",
            fixture.0.to_str().unwrap(),
        ],
        Some(source),
    );
    assert_eq!(code, 0);
    assert_eq!(value["data"]["resources"][0]["status"], "available");
    assert_eq!(value["data"]["resources"][0]["bytes"], 10);
    assert!(has_diagnostic(&value, "ExternalResource"));
    let source = "![missing](missing.png)\n\n![blocked](../outside.png)\n";
    let (code, value) = fixture.invoke(&["validate", "--stdin"], Some(source));
    assert_eq!(code, 1);
    assert!(has_diagnostic(&value, "MissingResource"));
    assert!(has_diagnostic(&value, "BlockedResource"));
    let source = "![picture][Example]\n\n[example]: missing.png\n";
    let (code, value) = fixture.invoke(&["validate", "--stdin"], Some(source));
    assert_eq!(code, 1);
    assert!(has_diagnostic(&value, "MissingResource"));
}

#[test]
fn run_captures_outputs_errors_timeout_and_visualization_using_the_source_directory() {
    if !python_available() {
        eprintln!("Python unavailable; skipping runtime integration.");
        return;
    }
    let fixture = Fixture::new();
    fs::write(fixture.0.join("input.txt"), "hello").unwrap();
    fixture.source("```python run id=hello visualize=array watch=values\nimport sys\nvalues = [1, 2]\nprint(open('input.txt').read())\nprint('{\"nested\": true}')\nprint('diagnostic', file=sys.stderr)\n```");
    let (code, value) = fixture.invoke(&["run", "lesson.ll.md", "--cell=hello"], None);
    assert_eq!(code, 0, "{value}");
    assert_eq!(
        value["data"]["result"]["stdout"]
            .as_str()
            .unwrap()
            .replace("\r\n", "\n"),
        "hello\n{\"nested\": true}\n"
    );
    assert_eq!(
        value["data"]["result"]["stderr"]
            .as_str()
            .unwrap()
            .replace("\r\n", "\n"),
        "diagnostic\n"
    );
    let trace = value["data"]["result"]["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["mime"] == "application/vnd.liveleaf.visualization+json")
        .unwrap();
    let trace: Value = serde_json::from_str(trace["value"].as_str().unwrap()).unwrap();
    assert!(!trace["frames"].as_array().unwrap().is_empty());
    assert_eq!(
        fixture
            .invoke(&["run", "lesson.ll.md", "--cell", "unknown"], None)
            .1["error"]["code"],
        "CellNotFound"
    );
    fixture.source("```python run id=fail\nraise ValueError('example failure')\n```");
    let (code, value) = fixture.invoke(&["run", "lesson.ll.md", "--cell", "fail"], None);
    assert_eq!(code, 1);
    assert_eq!(value["data"]["result"]["error"]["name"], "ValueError");
    fixture.source("```python run id=slow timeout=80\nwhile True:\n    pass\n```");
    let (code, value) = fixture.invoke(&["run", "lesson.ll.md", "--cell", "slow"], None);
    assert_eq!(code, 1);
    assert_eq!(value["data"]["result"]["status"], "timeout");
}

#[test]
fn pack_produces_a_native_package_and_protects_existing_files() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("image.png"), b"image").unwrap();
    let source = "# Package\n\n![image][Picture]\n\n[picture]: image.png\n\n```python run id=no-execution\nopen('executed.txt', 'w').write('bad')\n```\n";
    fixture.source(source);
    let (code, value) = fixture.invoke(
        &["pack", "lesson.ll.md", "--output", "nested/lesson.llpkg"],
        None,
    );
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["data"]["package"]["bundledAssetCount"], 1);
    let path = fixture.0.join("nested/lesson.llpkg");
    let mut zip = zip::ZipArchive::new(fs::File::open(&path).unwrap()).unwrap();
    let manifest: Value = serde_json::from_reader(zip.by_name("manifest.json").unwrap()).unwrap();
    assert_eq!(manifest["format"], "liveleaf-project");
    let mut document = String::new();
    zip.by_name("document.ll.md")
        .unwrap()
        .read_to_string(&mut document)
        .unwrap();
    assert_eq!(document, source);
    let mut image = Vec::new();
    zip.by_name(manifest["assets"][0]["packagePath"].as_str().unwrap())
        .unwrap()
        .read_to_end(&mut image)
        .unwrap();
    assert_eq!(image, b"image");
    drop(zip);
    assert!(!fixture.0.join("executed.txt").exists());
    assert_eq!(
        fixture
            .invoke(
                &["pack", "lesson.ll.md", "--output", "nested/lesson.llpkg"],
                None
            )
            .0,
        2
    );
    assert_eq!(
        fixture
            .invoke(
                &[
                    "pack",
                    "lesson.ll.md",
                    "--output",
                    "lesson.ll.md",
                    "--force"
                ],
                None
            )
            .0,
        2
    );
    assert_eq!(
        fs::read_to_string(fixture.0.join("lesson.ll.md")).unwrap(),
        source
    );
    fixture.source("# Updated\n");
    assert_eq!(
        fixture
            .invoke(
                &[
                    "pack",
                    "lesson.ll.md",
                    "-o",
                    "nested/lesson.llpkg",
                    "--force"
                ],
                None
            )
            .0,
        0
    );
    fixture.source("![missing](missing.png)\n");
    assert_eq!(
        fixture
            .invoke(&["pack", "lesson.ll.md", "-o", "invalid.llpkg"], None)
            .0,
        1
    );
    assert!(!fixture.0.join("invalid.llpkg").exists());
}
