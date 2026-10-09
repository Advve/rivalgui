//! `rivalcfg.handlers.choice`

use serde_json::Value;

use crate::core::profile::Setting;
use crate::core::pyval::{py_str, Result};

/// Choice keys ordered like `choices_to_list`: strings first, then numbers ascending.
pub fn ordered_keys(setting: &Setting) -> Vec<Value> {
    let mut keys: Vec<Value> = setting
        .choices
        .iter()
        .flatten()
        .map(|c| c.key.clone())
        .collect();
    keys.sort_by_key(|k| k.as_i64().unwrap_or(-1));
    keys
}

pub fn process_value(setting: &Setting, choice: &Value) -> Result<Vec<i64>> {
    let wanted = py_str(choice);
    let choices = setting.choices.as_deref().unwrap_or_default();
    let found = choices.iter().find(|c| py_str(&c.key) == wanted).ok_or_else(|| {
        let list: Vec<String> = ordered_keys(setting).iter().map(py_str).collect();
        format!("value must be one of [{}]", list.join(", "))
    })?;
    match &found.value {
        Value::Array(items) => items
            .iter()
            .map(|v| v.as_i64().ok_or_else(|| format!("invalid choice value {v}")))
            .collect(),
        v => Ok(vec![v.as_i64().ok_or_else(|| format!("invalid choice value {v}"))?]),
    }
}
