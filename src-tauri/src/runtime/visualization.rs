use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::Read;
use std::path::Path;

use super::graph::{self, GraphConfig, GraphState};
use super::table::{self, DynamicTableSnapshot, DynamicTableState};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MIME: &str = "application/vnd.liveleaf.visualization+json";
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const ATTRIBUTES: &[&str] = &[
    "visualize",
    "watch",
    "pointers",
    "scope",
    "value",
    "children",
    "node_states",
    "edge_states",
    "tables",
    "distances",
    "visited",
    "predecessors",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualizationConfig {
    #[serde(rename = "type")]
    pub kind: String,
    pub watch: String,
    pub pointers: Vec<String>,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree: Option<TreeConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<GraphConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tables: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TreeConfig {
    pub value_field: String,
    pub child_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VisualizationDiagnostic {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<usize>,
}

impl VisualizationDiagnostic {
    fn config(attribute: &str, message: String) -> Self {
        Self {
            code: "VisualizationConfigError".into(),
            message,
            attribute: Some(attribute.into()),
            seq: None,
        }
    }
}

fn identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !"False None True and as assert async await break class continue def del elif else except finally for from global if import in is lambda nonlocal not or pass raise return try while with yield".split(' ').any(|k| k == value)
}

fn unique_names(values: &[String], empty: bool) -> bool {
    (empty || !values.is_empty())
        && values.iter().all(|v| identifier(v))
        && values.iter().collect::<HashSet<_>>().len() == values.len()
}

pub fn validate_config(c: &VisualizationConfig) -> Result<(), String> {
    if !["array", "tree", "graph", "dynamic-table"].contains(&c.kind.as_str())
        || !identifier(&c.watch)
        || !identifier(&c.scope)
        || !unique_names(&c.pointers, true)
        || c.pointers.len() > 8
        || (c.kind == "dynamic-table" && !c.pointers.is_empty())
        || c.tables.as_ref().is_some_and(|t| !unique_names(t, true))
    {
        return Err("Invalid visualization type, watch, scope or pointers.".into());
    }
    match (&*c.kind, &c.tree, &c.graph) {
        ("tree", Some(t), None)
            if identifier(&t.value_field) && unique_names(&t.child_fields, false) =>
        {
            Ok(())
        }
        ("array" | "dynamic-table", None, None) => Ok(()),
        ("graph", None, Some(g))
            if g.variables()
                .iter()
                .all(|(_, value)| value.as_ref().is_none_or(|v| identifier(v))) =>
        {
            Ok(())
        }
        _ => Err("Invalid tree/graph fields for the selected visualization type.".into()),
    }
}

pub fn parse_config(
    language: &str,
    attrs: &HashMap<String, String>,
    duplicates: &HashSet<String>,
    flags: &HashSet<String>,
) -> (Option<VisualizationConfig>, Vec<VisualizationDiagnostic>) {
    if !ATTRIBUTES
        .iter()
        .any(|a| attrs.contains_key(*a) || flags.contains(*a))
    {
        return (None, vec![]);
    }
    let mut errors = vec![];
    let mut fail = |a: &str, m: String| errors.push(VisualizationDiagnostic::config(a, m));
    for a in ATTRIBUTES {
        if duplicates.contains(*a) {
            fail(a, format!("Duplicate attribute: {a}"));
        }
        if flags.contains(*a) {
            fail(a, format!("Attribute requires a value: {a}"));
        }
    }
    if !language.eq_ignore_ascii_case("python") {
        fail("visualize", "Visualization requires Python.".into());
    }
    let kind = attrs.get("visualize").cloned().unwrap_or_default();
    if !["array", "tree", "graph", "dynamic-table"].contains(&kind.as_str()) {
        fail(
            "visualize",
            "visualize must be array, tree, graph or dynamic-table.".into(),
        );
    }
    let mut name = |a: &str, fallback: &str| {
        let value = attrs
            .get(a)
            .map(String::as_str)
            .unwrap_or(fallback)
            .to_string();
        if !identifier(&value) {
            fail(a, format!("{a} must be an ASCII Python identifier."));
        }
        value
    };
    let watch = name("watch", "");
    let scope = name("scope", "module");
    let value_field = if kind == "tree" {
        Some(name("value", "value"))
    } else {
        None
    };
    let graph = if kind == "graph" {
        Some(GraphConfig {
            node_states: attrs.get("node_states").map(|_| name("node_states", "")),
            edge_states: attrs.get("edge_states").map(|_| name("edge_states", "")),
        })
    } else {
        None
    };
    let mut list = |a: &str, fallback: &str| {
        let raw = attrs.get(a).map(String::as_str).unwrap_or(fallback);
        let empty = a == "pointers" || a == "tables";
        let parts: Vec<String> = if empty && raw.is_empty() {
            vec![]
        } else {
            raw.split(',').map(|v| v.trim().to_string()).collect()
        };
        if !unique_names(&parts, empty) {
            fail(
                a,
                format!("{a} contains an empty, duplicate or invalid name."),
            );
        }
        parts
    };
    let pointers = list("pointers", "");
    let tables = attrs.get("tables").map(|_| list("tables", ""));
    let tree = value_field.map(|value_field| TreeConfig {
        value_field,
        child_fields: list("children", "left,right"),
    });
    if pointers.len() > 8 {
        fail("pointers", "At most 8 pointers are supported.".into());
    }
    if kind == "dynamic-table" && !pointers.is_empty() {
        fail("pointers", "dynamic-table does not use pointers.".into());
    }
    if kind != "tree" {
        for a in ["value", "children"] {
            if attrs.contains_key(a) {
                fail(a, format!("{a} is only supported for tree."));
            }
        }
    }
    if kind != "graph" {
        for a in ["node_states", "edge_states"] {
            if attrs.contains_key(a) {
                fail(a, format!("{a} is only supported for graph."));
            }
        }
    }
    for a in ["distances", "visited", "predecessors"] {
        if attrs.contains_key(a) {
            fail(a, format!("{a} has been replaced by node_states. Put algorithm fields in the node state dictionary."));
        }
    }
    let config = VisualizationConfig {
        kind,
        watch,
        pointers,
        scope,
        tree,
        graph,
        tables,
    };
    (
        if errors.is_empty() {
            Some(config)
        } else {
            None
        },
        errors,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrayState {
    pub object_id: String,
    pub items: Vec<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrayPointer {
    pub name: String,
    pub raw: Value,
    pub index: Option<usize>,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEdge {
    pub field: String,
    pub index: Option<usize>,
    pub node_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeNode {
    pub id: String,
    pub value: Value,
    pub children: Vec<TreeEdge>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeState {
    pub root_id: Option<String>,
    pub nodes: Vec<TreeNode>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreePointer {
    pub name: String,
    pub node_id: Option<String>,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TracePosition {
    pub cell_line: usize,
    pub function_name: String,
    pub frame_id: String,
    pub scope_frame_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_line: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FrameState {
    Array {
        state: Option<ArrayState>,
        pointers: Vec<ArrayPointer>,
    },
    Tree {
        state: Option<TreeState>,
        pointers: Vec<TreePointer>,
    },
    Graph {
        state: Option<GraphState>,
        pointers: Vec<TreePointer>,
    },
    #[serde(rename = "dynamic-table")]
    DynamicTable {
        state: Option<DynamicTableState>,
        pointers: Vec<TreePointer>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualizationFrame {
    pub seq: usize,
    pub event: String,
    pub position: TracePosition,
    pub status: String,
    pub diagnostic_codes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tables: Vec<DynamicTableSnapshot>,
    #[serde(flatten)]
    pub data: FrameState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capture {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualizationTrace {
    pub schema_version: u8,
    pub run_id: String,
    pub cell_id: String,
    pub config: VisualizationConfig,
    pub frames: Vec<VisualizationFrame>,
    pub capture: Capture,
    pub diagnostics: Vec<VisualizationDiagnostic>,
}

pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 256
}
fn decimal(s: &str) -> bool {
    let digits = s.strip_prefix('-').unwrap_or(s);
    !digits.is_empty()
        && s.len() <= 256 * 1024
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits.len() == 1 || !digits.starts_with('0'))
}
pub(super) fn scalar(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(_) => true,
        Value::String(s) => s.chars().count() <= 256,
        Value::Number(n) => {
            if n.is_f64() {
                n.as_f64().is_some_and(f64::is_finite)
            } else {
                n.as_i64()
                    .is_some_and(|v| v.unsigned_abs() <= 9_007_199_254_740_991)
            }
        }
        Value::Object(o) if o.len() == 2 => match o.get("kind").and_then(Value::as_str) {
            Some("integer") => o
                .get("decimal")
                .and_then(Value::as_str)
                .is_some_and(decimal),
            Some("float") => o
                .get("text")
                .and_then(Value::as_str)
                .is_some_and(|s| ["nan", "inf", "-inf"].contains(&s)),
            _ => false,
        },
        _ => false,
    }
}

fn validate_frame(
    f: &VisualizationFrame,
    config: &VisualizationConfig,
    seq: usize,
    line_count: usize,
) -> bool {
    let p = &f.position;
    if f.seq != seq
        || !["call", "line", "return", "exception"].contains(&f.event.as_str())
        || !["ready", "unbound", "invalid"].contains(&f.status.as_str())
        || p.cell_line == 0
        || p.cell_line > line_count.max(1)
        || !valid_id(&p.frame_id)
        || !valid_id(&p.scope_frame_id)
        || !valid_id(&p.function_name)
        || f.diagnostic_codes.len() > 32
        || !f.diagnostic_codes.iter().all(|s| valid_id(s))
    {
        return false;
    }
    let names = config.tables.as_deref().unwrap_or(&[]);
    if f.tables.len() != names.len()
        || !f.tables.iter().zip(names).all(|(t, name)| {
            &t.name == name
                && match (&*t.status, &t.state) {
                    ("ready", Some(s)) => table::validate_state(s),
                    ("unbound" | "invalid", None) => true,
                    _ => false,
                }
        })
    {
        return false;
    }
    match &f.data {
        FrameState::DynamicTable { state, pointers } => {
            config.kind == "dynamic-table"
                && (f.status == "ready") == state.is_some()
                && pointers.is_empty()
                && state.as_ref().is_none_or(table::validate_state)
        }
        FrameState::Array { state, pointers } => {
            if config.kind != "array"
                || (f.status == "ready") != state.is_some()
                || pointers.len() != config.pointers.len()
            {
                return false;
            }
            if let Some(s) = state {
                if !valid_id(&s.object_id) || s.items.len() > 256 || !s.items.iter().all(scalar) {
                    return false;
                }
            }
            pointers.iter().zip(&config.pointers).all(|(p, name)| {
                if &p.name != name || !scalar(&p.raw) {
                    return false;
                }
                match p.status.as_str() {
                    "inBounds" => state.as_ref().is_some_and(|s| {
                        p.index.is_some_and(|i| {
                            let n = s.items.len() as i64;
                            p.raw.as_i64().is_some_and(|v| {
                                n > 0 && v >= -n && v < n && i == v.rem_euclid(n) as usize
                            })
                        })
                    }),
                    "outOfBounds" => {
                        p.index.is_none()
                            && (p.raw.as_i64().is_some()
                                || p.raw.get("kind").and_then(Value::as_str) == Some("integer"))
                    }
                    "unbound" => p.index.is_none() && p.raw.is_null(),
                    "invalidType" => p.index.is_none(),
                    _ => false,
                }
            })
        }
        FrameState::Graph { state, pointers } => {
            if config.kind != "graph"
                || (f.status == "ready") != state.is_some()
                || pointers.len() != config.pointers.len()
            {
                return false;
            }
            if let Some(s) = state {
                if !graph::validate_state(s, config.graph.as_ref().unwrap()) {
                    return false;
                }
            }
            let ids: HashSet<_> = state
                .as_ref()
                .map(|s| s.nodes.iter().map(|n| n.id.as_str()).collect())
                .unwrap_or_default();
            pointers.iter().zip(&config.pointers).all(|(p, name)| {
                &p.name == name
                    && match p.status.as_str() {
                        "matched" => p
                            .node_id
                            .as_ref()
                            .is_some_and(|id| ids.contains(id.as_str())),
                        "null" | "outside" | "unbound" | "invalidType" => p.node_id.is_none(),
                        _ => false,
                    }
            })
        }
        FrameState::Tree { state, pointers } => {
            if config.kind != "tree"
                || (f.status == "ready") != state.is_some()
                || pointers.len() != config.pointers.len()
            {
                return false;
            }
            let mut ids = HashSet::new();
            if let Some(s) = state {
                if s.nodes.len() > 128
                    || s.nodes.iter().any(|n| {
                        !valid_id(&n.id) || !ids.insert(n.id.as_str()) || !scalar(&n.value)
                    })
                {
                    return false;
                }
                let fields = &config.tree.as_ref().unwrap().child_fields;
                let mut edge_count = 0;
                for n in &s.nodes {
                    let mut last_field = None;
                    let mut last_index = None;
                    for e in &n.children {
                        edge_count += 1;
                        let Some(field_index) = fields.iter().position(|f| f == &e.field) else {
                            return false;
                        };
                        if edge_count > 512
                            || e.node_id
                                .as_ref()
                                .is_some_and(|id| !ids.contains(id.as_str()))
                            || e.index.is_some_and(|i| i >= 512)
                        {
                            return false;
                        }
                        if let Some(prev) = last_field {
                            if field_index < prev
                                || (field_index == prev
                                    && (last_index.is_none()
                                        || e.index != last_index.map(|i| i + 1)))
                            {
                                return false;
                            }
                        }
                        if last_field != Some(field_index) && e.index.is_some_and(|i| i != 0) {
                            return false;
                        }
                        last_field = Some(field_index);
                        last_index = e.index;
                    }
                }
                let map: HashMap<_, _> = s.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
                let mut seen = HashSet::new();
                let mut queue = VecDeque::new();
                if let Some(root) = &s.root_id {
                    queue.push_back((root.as_str(), 0));
                }
                while let Some((key, depth)) = queue.pop_front() {
                    let Some(node) = map.get(key) else {
                        return false;
                    };
                    if !seen.insert(key) || depth > 32 {
                        return false;
                    }
                    for edge in &node.children {
                        if let Some(id) = &edge.node_id {
                            queue.push_back((id.as_str(), depth + 1));
                        }
                    }
                }
                if seen.len() != s.nodes.len() {
                    return false;
                }
            }
            pointers.iter().zip(&config.pointers).all(|(p, name)| {
                &p.name == name
                    && match p.status.as_str() {
                        "matched" => p
                            .node_id
                            .as_ref()
                            .is_some_and(|id| ids.contains(id.as_str())),
                        "null" | "outside" | "unbound" | "invalidType" => p.node_id.is_none(),
                        _ => false,
                    }
            })
        }
    }
}

pub fn read_trace(
    path: &Path,
    run_id: &str,
    cell_id: &str,
    config: &VisualizationConfig,
    line_count: usize,
    process_status: &str,
    source_start_line: Option<usize>,
) -> VisualizationTrace {
    let mut trace = VisualizationTrace {
        schema_version: 1,
        run_id: run_id.into(),
        cell_id: cell_id.into(),
        config: config.clone(),
        frames: vec![],
        diagnostics: vec![],
        capture: Capture {
            status: "error".into(),
            reason: Some("missingTrace".into()),
        },
    };
    let read = || -> Result<Vec<u8>, std::io::Error> {
        let file = fs::File::open(path)?;
        if file.metadata()?.len() > MAX_BYTES as u64 {
            return Err(std::io::Error::other("trace exceeds byte budget"));
        }
        let mut data = vec![];
        file.take(MAX_BYTES as u64 + 1).read_to_end(&mut data)?;
        Ok(data)
    };
    let Ok(data) = read() else {
        return trace;
    };
    if data.len() > MAX_BYTES {
        return trace;
    }
    let mut header = false;
    let mut ended = false;
    let mut corrupt = false;
    let mut objects = HashSet::new();
    for line in data.split_inclusive(|b| *b == b'\n') {
        if !line.ends_with(b"\n") {
            break;
        }
        if ended || line.len() > 256 * 1024 {
            corrupt = true;
            break;
        }
        let Ok(record) = serde_json::from_slice::<Value>(line) else {
            corrupt = true;
            break;
        };
        if !header {
            if record["kind"] != "header"
                || record["schemaVersion"] != 1
                || record["runId"] != run_id
                || record["cellId"] != cell_id
                || serde_json::from_value::<VisualizationConfig>(record["config"].clone())
                    .ok()
                    .as_ref()
                    != Some(config)
            {
                trace.capture.reason = Some("invalidHeader".into());
                return trace;
            }
            header = true;
            continue;
        }
        match record["kind"].as_str() {
            Some("frame") => {
                let Ok(mut frame) =
                    serde_json::from_value::<VisualizationFrame>(record["frame"].clone())
                else {
                    corrupt = true;
                    break;
                };
                if trace.frames.len() >= 2000
                    || !validate_frame(&frame, config, trace.frames.len(), line_count)
                {
                    corrupt = true;
                    break;
                }
                match &frame.data {
                    FrameState::Array { state: Some(s), .. } => {
                        objects.insert(s.object_id.clone());
                    }
                    FrameState::Tree { state: Some(s), .. } => {
                        objects.extend(s.nodes.iter().map(|n| n.id.clone()));
                    }
                    FrameState::Graph { state: Some(s), .. } => {
                        objects.insert(s.object_id.clone());
                        objects.extend(s.nodes.iter().map(|n| n.id.clone()));
                    }
                    FrameState::DynamicTable { state: Some(s), .. } => {
                        objects.insert(s.object_id.clone());
                    }
                    _ => {}
                }
                objects.extend(
                    frame
                        .tables
                        .iter()
                        .filter_map(|t| t.state.as_ref().map(|s| s.object_id.clone())),
                );
                if objects.len() > 512 {
                    corrupt = true;
                    break;
                }
                frame.position.document_line =
                    source_start_line.and_then(|start| start.checked_add(frame.position.cell_line));
                trace.frames.push(frame);
            }
            Some("diagnostic") => {
                let Ok(d) =
                    serde_json::from_value::<VisualizationDiagnostic>(record["diagnostic"].clone())
                else {
                    corrupt = true;
                    break;
                };
                if trace.diagnostics.len() >= 32
                    || !valid_id(&d.code)
                    || d.message.len() > 4096
                    || d.seq.is_some_and(|s| s != trace.frames.len())
                {
                    corrupt = true;
                    break;
                }
                trace.diagnostics.push(d);
            }
            Some("captureEnd") => {
                let Ok(capture) = serde_json::from_value::<Capture>(record["capture"].clone())
                else {
                    corrupt = true;
                    break;
                };
                if !["complete", "truncated", "error"].contains(&capture.status.as_str())
                    || capture.reason.as_ref().is_some_and(|s| !valid_id(s))
                {
                    corrupt = true;
                    break;
                }
                trace.capture = capture;
                ended = true;
            }
            _ => {
                corrupt = true;
                break;
            }
        }
    }
    if !header {
        trace.capture.reason = Some("invalidHeader".into());
    } else if corrupt {
        trace.capture = Capture {
            status: "error".into(),
            reason: Some("invalidRecord".into()),
        };
    } else if !ended {
        trace.capture = Capture {
            status: if process_status == "success" {
                "error"
            } else {
                "truncated"
            }
            .into(),
            reason: Some(
                if process_status == "timeout" {
                    "timeout"
                } else {
                    "missingCaptureEnd"
                }
                .into(),
            ),
        };
    }
    trace
}

pub fn write_runner(run_dir: &Path) -> Result<(), String> {
    let package = run_dir.join("liveleaf_viz");
    fs::create_dir_all(&package).map_err(|e| e.to_string())?;
    for (name, content) in [
        ("runner.py", include_str!("../../python/runner.py")),
        (
            "liveleaf_viz/__init__.py",
            include_str!("../../python/liveleaf_viz/__init__.py"),
        ),
        (
            "liveleaf_viz/values.py",
            include_str!("../../python/liveleaf_viz/values.py"),
        ),
        (
            "liveleaf_viz/array.py",
            include_str!("../../python/liveleaf_viz/array.py"),
        ),
        (
            "liveleaf_viz/tree.py",
            include_str!("../../python/liveleaf_viz/tree.py"),
        ),
        (
            "liveleaf_viz/graph.py",
            include_str!("../../python/liveleaf_viz/graph.py"),
        ),
        (
            "liveleaf_viz/table.py",
            include_str!("../../python/liveleaf_viz/table.py"),
        ),
        (
            "liveleaf_viz/collector.py",
            include_str!("../../python/liveleaf_viz/collector.py"),
        ),
    ] {
        fs::write(run_dir.join(name), content).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config() -> VisualizationConfig {
        serde_json::from_value(json!({"type":"array","watch":"arr","pointers":[],"scope":"module"}))
            .unwrap()
    }

    fn header() -> Value {
        json!({"kind":"header","schemaVersion":1,"runId":"run","cellId":"cell","config":config()})
    }
    fn frame(seq: usize) -> Value {
        json!({"kind":"frame","frame":{"seq":seq,"event":"line","position":{"cellLine":1,"functionName":"<module>","frameId":"f1","scopeFrameId":"f1"},"status":"ready","diagnosticCodes":[],"type":"array","state":{"objectId":"o1","items":[1]},"pointers":[]}})
    }
    fn read(records: &[Value], tail: &str, status: &str) -> VisualizationTrace {
        let path = std::env::temp_dir().join(format!(
            "liveleaf-trace-test-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut data = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap() + "\n")
            .collect::<String>();
        data.push_str(tail);
        fs::write(&path, data).unwrap();
        let trace = read_trace(&path, "run", "cell", &config(), 1, status, None);
        fs::remove_file(path).unwrap();
        trace
    }

    #[test]
    fn recovers_only_continuous_valid_prefix() {
        let trace = read(&[header(), frame(0)], "{\"kind\":\"frame\"", "timeout");
        assert_eq!(trace.frames.len(), 1);
        assert_eq!(trace.capture.status, "truncated");
        assert_eq!(trace.capture.reason.as_deref(), Some("timeout"));
        let trace = read(&[header(), frame(0), frame(2), frame(1)], "", "error");
        assert_eq!(trace.frames.len(), 1);
        assert_eq!(trace.capture.reason.as_deref(), Some("invalidRecord"));
        let trace = read(&[header(), frame(0)], "invalid json\n", "success");
        assert_eq!(trace.frames.len(), 1);
        assert_eq!(trace.capture.status, "error");
    }

    #[test]
    fn rejects_unknown_version_wrong_run_and_missing_end() {
        for (field, value) in [
            ("schemaVersion", json!(2)),
            ("runId", json!("wrong")),
            ("cellId", json!("wrong")),
        ] {
            let mut h = header();
            h[field] = value;
            let trace = read(&[h, frame(0)], "", "success");
            assert!(trace.frames.is_empty());
            assert_eq!(trace.capture.reason.as_deref(), Some("invalidHeader"));
        }
        assert_eq!(
            read(&[header(), frame(0)], "", "success").capture.status,
            "error"
        );
        assert_eq!(
            read(&[header(), frame(0)], "", "error").capture.status,
            "truncated"
        );
    }

    #[test]
    fn rejects_bad_scalars_and_ready_state_mismatches() {
        for value in [
            json!(9007199254740992u64),
            json!({"kind":"float","text":"Infinity"}),
            json!({"unsupported":"x"}),
            json!("a".repeat(257)),
        ] {
            let mut f = frame(0);
            f["frame"]["state"]["items"] = json!([value]);
            assert!(read(&[header(), f], "", "success").frames.is_empty());
        }
        let mut f = frame(0);
        f["frame"]["status"] = json!("invalid");
        assert!(read(&[header(), f], "", "success").frames.is_empty());
    }

    #[test]
    fn tree_validation_rejects_dangling_cycles_and_unreachable_nodes() {
        let config: VisualizationConfig = serde_json::from_value(json!({"type":"tree","watch":"root","pointers":[],"scope":"module","tree":{"valueField":"val","childFields":["left","right"]}})).unwrap();
        let f = json!({"seq":0,"event":"line","position":{"cellLine":1,"functionName":"<module>","frameId":"f1","scopeFrameId":"f1"},"status":"ready","diagnosticCodes":[],"type":"tree","state":{"rootId":"o1","nodes":[{"id":"o1","value":1,"children":[{"field":"left","index":null,"nodeId":null},{"field":"right","index":null,"nodeId":null}]}]},"pointers":[]});
        let valid = |v: Value| validate_frame(&serde_json::from_value(v).unwrap(), &config, 0, 1);
        assert!(valid(f.clone()));
        let mut dangling = f.clone();
        dangling["state"]["nodes"][0]["children"][0]["nodeId"] = json!("missing");
        assert!(!valid(dangling));
        let mut cycle = f.clone();
        cycle["state"]["nodes"][0]["children"][0]["nodeId"] = json!("o1");
        assert!(!valid(cycle));
        let mut disconnected = f;
        disconnected["state"]["rootId"] = Value::Null;
        assert!(!valid(disconnected));
    }

    #[test]
    fn table_bindings_are_validated_independently_of_primary_state() {
        let config: VisualizationConfig = serde_json::from_value(json!({"type":"graph","watch":"graph","pointers":[],"scope":"module","graph":{},"tables":["rows","queue"]})).unwrap();
        let frame = json!({"seq":0,"event":"line","position":{"cellLine":1,"functionName":"<module>","frameId":"f1","scopeFrameId":"f1"},"status":"invalid","diagnosticCodes":[],"type":"graph","state":null,"pointers":[],"tables":[{"name":"rows","status":"ready","state":{"objectId":"rows1","columns":["cost"],"rows":[[3]]}},{"name":"queue","status":"unbound","state":null}]});
        let valid = |v| {
            serde_json::from_value::<VisualizationFrame>(v)
                .is_ok_and(|f| validate_frame(&f, &config, 0, 1))
        };
        assert!(valid(frame.clone()));
        for (path, value) in [
            ("/tables/0/name", json!("queue")),
            ("/tables/0/status", json!("invalid")),
            ("/tables/0/state/rows", json!([[]])),
            ("/tables", json!([])),
        ] {
            let mut bad = frame.clone();
            *bad.pointer_mut(path).unwrap() = value;
            assert!(!valid(bad), "{path}");
        }
    }
}
