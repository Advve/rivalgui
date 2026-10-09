//! `rivalcfg.handlers.range_choice`, `multidpi_range_choice` and
//! `multidpi_range_choice_xy` (rivalcfg >= 4.16): DPI values are looked up in
//! a per-sensor table instead of being computed.

use serde_json::Value;

use crate::core::profile::Setting;
use crate::core::pyval::{parse_int, py_int, uint_to_le, Result};

use super::range::parse_dpi_list;

/// `find_nearest_choice`: ties go to the lowest value.
fn find_nearest_choice(choices: &[(i64, i64)], value: i64) -> Option<(i64, i64)> {
    let mut sorted = choices.to_vec();
    sorted.sort_by_key(|(k, _)| *k);
    let mut best: Option<((i64, i64), i64)> = None;
    for choice in sorted {
        let delta = (choice.0 - value).abs();
        if best.is_none_or(|(_, d)| delta < d) {
            best = Some((choice, delta));
        }
    }
    best.map(|(c, _)| c)
}

/// `process_range_choice`: maps a DPI to the device value of the nearest table entry.
pub fn process_range_choice(setting: &Setting, value: i64) -> Result<i64> {
    let [first, last, step] = setting.input_range.ok_or("missing input_range")?;
    let choices = setting.output_choices.as_deref().ok_or("missing output_choices")?;
    if step == 0 || choices.len() as f64 != (last - first + step) as f64 / step as f64 {
        return Err("Input range and output choices mismatch: not the same length".into());
    }
    if choices.iter().map(|(k, _)| *k).min() != Some(first) {
        return Err("Input range and output choices mismatch: not the same min value".into());
    }
    if choices.iter().map(|(k, _)| *k).max() != Some(last) {
        return Err("Input range and output choices mismatch: not the same max value".into());
    }
    let (_, out) = find_nearest_choice(choices, value).ok_or("no choice")?;
    Ok(out)
}

pub fn process_value(setting: &Setting, value: &Value) -> Result<Vec<i64>> {
    let length = setting.range_length_byte.unwrap_or(1);
    uint_to_le(process_range_choice(setting, py_int(value)?)?, length)
}

/// Shared preset checks of the multi-DPI handlers. Returns the selected preset.
fn check_presets(setting: &Setting, count: usize, selected_preset: Option<i64>) -> Result<i64> {
    let first_preset = setting.first_preset.unwrap_or(0);
    let selected = selected_preset.map_or(first_preset, |p| p + first_preset);
    let max = setting.max_preset_count.unwrap_or(1);
    if count == 0 {
        return Err("you must provide at least one preset".into());
    }
    if count > max {
        return Err(format!(
            "you provided {count} preset but the device accepts a maximum of {max} presets"
        ));
    }
    if !(first_preset <= selected && selected < count as i64 + first_preset) {
        return Err("the selected preset is out of range".into());
    }
    Ok(selected)
}

pub fn process_multidpi(setting: &Setting, value: &Value, selected_preset: Option<i64>) -> Result<Vec<i64>> {
    let dpis = parse_dpi_list(value)?;
    let selected = check_presets(setting, dpis.len(), selected_preset)?;
    let dpi_length = setting
        .dpi_length_byte
        .ok_or("Missing 'dpi_length_byte' parameter for 'multidpi_range_choice' handler")?;
    let mut out = vec![dpis.len() as i64, selected];
    for dpi in dpis {
        out.extend(uint_to_le(process_range_choice(setting, dpi)?, dpi_length)?);
    }
    Ok(out)
}

/// `normalize_value`: accepts `800`, `[800, 1600]`, `[[800, 900], 1600]`, `"800:900, 1600"`.
pub fn normalize_xy(value: &Value) -> Result<Vec<[i64; 2]>> {
    let pair = |items: &[Value]| -> Result<[i64; 2]> {
        match items {
            [x] => Ok([py_int(x)?, py_int(x)?]),
            [x, y] => Ok([py_int(x)?, py_int(y)?]),
            _ => Err(format!("invalid DPI pair {}", Value::from(items.to_vec()))),
        }
    };
    match value {
        Value::Number(_) | Value::Bool(_) => {
            let v = py_int(value)?;
            Ok(vec![[v, v]])
        }
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Number(_) | Value::Bool(_) => {
                    let v = py_int(item)?;
                    Ok([v, v])
                }
                Value::Array(inner) => pair(inner),
                other => Err(format!("invalid DPI value {other}")),
            })
            .collect(),
        Value::String(s) => s
            .replace(' ', "")
            .split(',')
            .map(|item| {
                let parts: Vec<&str> = item.split(':').collect();
                match parts.as_slice() {
                    [x] => {
                        let v = parse_int(x)?;
                        Ok([v, v])
                    }
                    [x, y] => Ok([parse_int(x)?, parse_int(y)?]),
                    _ => Err(format!("invalid DPI value '{item}'")),
                }
            })
            .collect(),
        other => Err(format!("invalid DPI value {other}")),
    }
}

pub fn process_multidpi_xy(setting: &Setting, value: &Value, selected_preset: Option<i64>) -> Result<Vec<i64>> {
    let dpis = normalize_xy(value)?;
    let selected = check_presets(setting, dpis.len(), selected_preset)?;
    let max = setting.max_preset_count.unwrap_or(1);
    let mapping = setting
        .xy_mapping
        .as_deref()
        .ok_or("Missing 'xy_mapping' parameter for 'multidpi_range_choice_xy' handler")?;
    let dpi_length = setting
        .dpi_length_byte
        .ok_or("Missing 'dpi_length_byte' parameter for 'multidpi_range_choice_xy' handler")?;

    let mut pairs = Vec::new();
    for [x, y] in &dpis {
        pairs.push((
            uint_to_le(process_range_choice(setting, *x)?, dpi_length)?,
            uint_to_le(process_range_choice(setting, *y)?, dpi_length)?,
        ));
    }

    let mut out = vec![dpis.len() as i64, selected];
    match mapping {
        "xyxy" => {
            for (x, y) in pairs {
                out.extend(x);
                out.extend(y);
            }
        }
        "xxyy" => {
            // Unused presets are zero-filled so Y values land at their fixed offset
            pairs.resize(max.max(pairs.len()), (vec![0; dpi_length], vec![0; dpi_length]));
            out.extend(pairs.iter().flat_map(|(x, _)| x.iter().copied()));
            out.extend(pairs.iter().flat_map(|(_, y)| y.iter().copied()));
        }
        other => return Err(format!("Unsupported value for xy-mapping: {other}")),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest() {
        let c = [(100, 1), (200, 2), (300, 3)];
        assert_eq!(find_nearest_choice(&c, 149), Some((100, 1)));
        assert_eq!(find_nearest_choice(&c, 150), Some((100, 1)));
        assert_eq!(find_nearest_choice(&c, 151), Some((200, 2)));
        assert_eq!(find_nearest_choice(&c, 9000), Some((300, 3)));
    }

    #[test]
    fn xy_normalization() {
        let v: Value = serde_json::from_str(r#"" 100:150, 200 ""#).unwrap();
        assert_eq!(normalize_xy(&v).unwrap(), vec![[100, 150], [200, 200]]);
        let v: Value = serde_json::from_str("[[100], [100, 150], 200]").unwrap();
        assert_eq!(normalize_xy(&v).unwrap(), vec![[100, 100], [100, 150], [200, 200]]);
        assert!(normalize_xy(&Value::from("1:2:3")).is_err());
    }
}
