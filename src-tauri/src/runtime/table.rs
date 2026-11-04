use super::visualization::{scalar, valid_id};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DynamicTableState {
    pub object_id: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynamicTableSnapshot {
    pub name: String,
    pub status: String,
    pub state: Option<DynamicTableState>,
}

pub fn validate_state(s: &DynamicTableState) -> bool {
    valid_id(&s.object_id)
        && s.rows.len() <= 256
        && s.columns
            .iter()
            .all(|c| !c.is_empty() && c.chars().count() <= 256)
        && s.columns.iter().collect::<HashSet<_>>().len() == s.columns.len()
        && s.rows
            .iter()
            .all(|row| row.len() == s.columns.len() && row.iter().all(scalar))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_columns_dimensions_and_scalars() {
        let state = json!({"objectId":"rows","columns":["节点","cost"],"rows":[["A",0],["B",{"kind":"float","text":"inf"}]]});
        let valid =
            |s| serde_json::from_value::<DynamicTableState>(s).is_ok_and(|s| validate_state(&s));
        assert!(valid(state.clone()));
        for (field, value) in [
            ("columns", json!(["cost", "cost"])),
            ("columns", json!([""])),
            ("rows", json!([[1]])),
            ("rows", json!([[{}, 0]])),
            ("rows", json!(vec![vec![0, 0]; 257])),
        ] {
            let mut bad = state.clone();
            bad[field] = value;
            assert!(!valid(bad), "{field}");
        }
    }
}
