//! `rivalcfg.handlers.rgbcolor` and `rivalcfg.handlers.reactive_rgbcolor`

use serde_json::Value;

use crate::core::pyval::{color_from_value, py_str, Result};

pub fn process_rgbcolor(value: &Value) -> Result<Vec<i64>> {
    Ok(color_from_value(value)?.iter().map(|c| *c as i64).collect())
}

pub fn is_reactive_off(value: &Value) -> bool {
    value.is_null() || matches!(py_str(value).to_lowercase().as_str(), "off" | "disable")
}

pub fn process_reactive(value: &Value) -> Result<Vec<i64>> {
    if is_reactive_off(value) {
        return Ok(vec![0x00; 5]);
    }
    let mut out = vec![0x01, 0x00];
    out.extend(color_from_value(value)?.iter().map(|c| *c as i64));
    Ok(out)
}
