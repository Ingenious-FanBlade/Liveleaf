use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::visualization::{scalar, valid_id};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_states: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_states: Option<String>,
}

impl GraphConfig {
    pub fn variables(&self) -> [(&str, &Option<String>); 2] {
        [
            ("nodeStates", &self.node_states),
            ("edgeStates", &self.edge_states),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub value: Value,
    pub state: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub weight: Value,
    pub state: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphAnnotations {
    pub node_states: String,
    pub edge_states: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphState {
    pub object_id: String,
    pub directed: bool,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub annotations: GraphAnnotations,
}

fn integer(v: &Value) -> bool {
    v.as_i64().is_some() || v.get("kind").and_then(Value::as_str) == Some("integer")
}

fn numeric(v: &Value) -> bool {
    v.is_number() || v.get("kind").and_then(Value::as_str) == Some("integer")
}

fn key(v: &Value) -> String {
    if let Some(s) = v.as_str() {
        return format!("s:{s}");
    }
    let s = v
        .get("decimal")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| v.to_string());
    format!("i:{}", if s == "-0" { "0" } else { &s })
}

pub fn validate_state(s: &GraphState, config: &GraphConfig) -> bool {
    if !valid_id(&s.object_id) || !s.directed || s.nodes.len() > 128 || s.edges.len() > 512 {
        return false;
    }
    let annotations = [
        (&s.annotations.node_states, &config.node_states),
        (&s.annotations.edge_states, &config.edge_states),
    ];
    if annotations.iter().any(|(status, variable)| {
        !["off", "ready", "unbound", "invalid"].contains(&status.as_str())
            || (status.as_str() == "off") != variable.is_none()
    }) {
        return false;
    }
    let mut ids = HashSet::new();
    let mut keys = HashSet::new();
    for n in &s.nodes {
        if !valid_id(&n.id)
            || !ids.insert(n.id.as_str())
            || !scalar(&n.value)
            || !(n.value.is_string() || integer(&n.value))
            || !keys.insert(key(&n.value))
            || !valid_members(&n.state, &s.annotations.node_states)
        {
            return false;
        }
    }
    let mut edge_ids = HashSet::new();
    let mut pairs = HashSet::new();
    for e in &s.edges {
        if !valid_id(&e.id)
            || !edge_ids.insert(e.id.as_str())
            || !ids.contains(e.source.as_str())
            || !ids.contains(e.target.as_str())
            || !pairs.insert((e.source.as_str(), e.target.as_str()))
            || !scalar(&e.weight)
            || !(e.weight.is_null() || numeric(&e.weight))
            || !valid_members(&e.state, &s.annotations.edge_states)
        {
            return false;
        }
    }
    true
}

fn valid_members(members: &Map<String, Value>, status: &str) -> bool {
    (status == "ready" || members.is_empty())
        && members
            .iter()
            .all(|(name, value)| !name.is_empty() && name.chars().count() <= 256 && scalar(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state() -> Value {
        json!({"objectId":"o1","directed":true,"nodes":[{"id":"a","value":"A","state":{"distance":0,"visited":true}},{"id":"b","value":"B","state":{"distance":2,"visited":false,"predecessor":"A"}},{"id":"c","value":"G","state":{"distance":{"kind":"float","text":"inf"}}}],"edges":[{"id":"ab","source":"a","target":"b","weight":2,"state":{"flow":1,"capacity":8}},{"id":"ba","source":"b","target":"a","weight":3,"state":{}},{"id":"aa","source":"a","target":"a","weight":0,"state":{}}],"annotations":{"nodeStates":"ready","edgeStates":"ready"}})
    }

    fn valid(value: Value) -> bool {
        let config = GraphConfig {
            node_states: Some("nodes".into()),
            edge_states: Some("edges".into()),
        };
        serde_json::from_value::<GraphState>(value)
            .is_ok_and(|state| validate_state(&state, &config))
    }

    #[test]
    fn permits_cycles_self_edges_and_disconnected_nodes() {
        assert!(valid(state()));
        let mut graph = state();
        graph["edges"][1]["weight"] = json!(-4);
        assert!(
            valid(graph),
            "General graph rendering permits finite negative weights"
        );
    }

    #[test]
    fn rejects_dangling_edges_duplicates_and_invalid_annotations() {
        for (path, value) in [
            ("/directed", json!(false)),
            ("/edges/0/target", json!("missing")),
            ("/edges/0/weight", json!(true)),
            ("/edges/0/weight", json!({"kind":"float","text":"inf"})),
            ("/nodes/0/value", json!(false)),
            ("/nodes/1/id", json!("a")),
            ("/nodes/1/state", json!({"nested":[]})),
            ("/edges/0/state", json!({"nested":{}})),
            ("/annotations/nodeStates", json!("unbound")),
            ("/annotations/edgeStates", json!("unbound")),
        ] {
            let mut graph = state();
            *graph.pointer_mut(path).unwrap() = value;
            assert!(!valid(graph), "{path}");
        }
        let mut graph = state();
        let mut edge = graph["edges"][0].clone();
        edge["id"] = json!("other");
        graph["edges"].as_array_mut().unwrap().push(edge);
        assert!(!valid(graph));
    }
}
