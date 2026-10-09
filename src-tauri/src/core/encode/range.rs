//! `rivalcfg.handlers.range` and `rivalcfg.handlers.multidpi_range`

use serde_json::Value;

use crate::core::profile::Setting;
use crate::core::pyval::{py_int, uint_to_le, Result};

/// Snaps `value` to the nearest step of the range (`matches_value_in_range`).
pub fn matches_value_in_range(start: i64, stop: i64, step: i64, value: i64) -> i64 {
    if value <= start {
        return start;
    }
    if value >= stop {
        return stop;
    }
    let delta = (value - start).rem_euclid(step);
    if delta == 0 {
        value
    } else if (delta as f64) < step as f64 / 2.0 {
        value - delta
    } else {
        value - delta + step
    }
}

/// Python `range(start, stop, step)` (integers only since rivalcfg 4.16).
fn int_range(start: i64, stop: i64, step: i64) -> Result<Vec<i64>> {
    if step <= 0 {
        return Err("range() arg 3 must be positive".into());
    }
    Ok((start..stop).step_by(step as usize).collect())
}

fn as_int(v: f64) -> Result<i64> {
    if v.fract() != 0.0 {
        return Err("'float' object cannot be interpreted as an integer".into());
    }
    Ok(v as i64)
}

/// `process_range`: maps an input value to the device's output scale.
pub fn process_range(setting: &Setting, value: i64) -> Result<i64> {
    let [in_start, in_stop, in_step] = setting.input_range.ok_or("missing input_range")?;
    let [out_start, out_stop, out_step] = setting.output_range.ok_or("missing output_range")?;
    let input = int_range(in_start, in_stop + 1, in_step)?;
    let output = int_range(as_int(out_start)?, as_int(out_stop)? + 1, as_int(out_step)?)?;
    if input.len() != output.len() {
        return Err("Input range and output range must have the same length".into());
    }
    let matched = matches_value_in_range(in_start, in_stop, in_step, value);
    let index = input
        .iter()
        .position(|v| *v == matched)
        .ok_or_else(|| format!("{matched} is not in list"))?;
    Ok(output[index])
}

pub fn process_value(setting: &Setting, value: &Value) -> Result<Vec<i64>> {
    let length = setting.range_length_byte.unwrap_or(1);
    uint_to_le(process_range(setting, py_int(value)?)?, length)
}

/// Parses a DPI list given as a number, a list or a "400, 800" string.
pub fn parse_dpi_list(value: &Value) -> Result<Vec<i64>> {
    match value {
        Value::Number(_) | Value::Bool(_) => Ok(vec![py_int(value)?]),
        Value::Array(items) => items.iter().map(py_int).collect(),
        Value::String(s) => s.replace(' ', "").split(',').map(crate::core::pyval::parse_int).collect(),
        other => Err(format!("invalid DPI value {other}")),
    }
}

pub fn process_multidpi(setting: &Setting, value: &Value, selected_preset: Option<i64>) -> Result<Vec<i64>> {
    let dpis = parse_dpi_list(value)?;
    let first_preset = setting.first_preset.unwrap_or(0);
    let selected = selected_preset.map_or(first_preset, |p| p + first_preset);
    let max = setting.max_preset_count.unwrap_or(1);

    if dpis.is_empty() {
        return Err("you must provide at least one preset".into());
    }
    if dpis.len() > max {
        return Err(format!(
            "you provided {} preset but the device accepts a maximum of {max} presets",
            dpis.len()
        ));
    }
    if !(first_preset <= selected && selected < dpis.len() as i64 + first_preset) {
        return Err("the selected preset is out of range".into());
    }
    let dpi_length = setting
        .dpi_length_byte
        .ok_or("Missing 'dpi_length_byte' parameter for 'multidpi_range' handler")?;

    let mut values = Vec::new();
    for dpi in &dpis {
        values.extend(uint_to_le(process_range(setting, *dpi)?, dpi_length)?);
    }
    let count = match setting.count_mode.as_deref() {
        Some("number") => dpis.len() as i64,
        Some("flag") => {
            if dpis.len() > 8 {
                return Err("negative shift count".into());
            }
            0b1111_1111 >> (8 - dpis.len())
        }
        other => return Err(format!("Invalid 'count_mode' parameter '{other:?}'")),
    };
    let mut out = vec![count, selected];
    out.extend(values);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping() {
        assert_eq!(matches_value_in_range(0, 100, 10, 42), 40);
        assert_eq!(matches_value_in_range(0, 1000, 100, 51), 100);
        assert_eq!(matches_value_in_range(42, 1000, 100, 150), 142);
        assert_eq!(matches_value_in_range(500, 1000, 100, 4000), 1000);
    }

    #[test]
    fn int_ranges_only() {
        assert_eq!(int_range(0, 5, 2).unwrap(), vec![0, 2, 4]);
        assert!(as_int(2.33).is_err());
    }
}
