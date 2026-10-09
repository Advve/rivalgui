//! `rivalcfg.handlers.buttons`

use std::collections::HashMap;

use serde_json::Value;

use crate::core::profile::{catalog, Setting};
use crate::core::pyval::{match_param_string, parse_param_string, Result};

const LAYOUTS: [&str; 1] = ["qwerty"];

/// Special actions available on this device (lowercase name -> code).
pub fn special_actions(setting: &Setting) -> Vec<(&'static str, u8)> {
    let mut out = Vec::new();
    if let Some(v) = setting.button_disable {
        out.push(("disable", v));
        out.push(("disabled", v));
    }
    if let Some(v) = setting.button_dpi_switch {
        out.push(("dpi", v));
    }
    if let Some(v) = setting.button_scroll_up {
        out.push(("scrollup", v));
    }
    if let Some(v) = setting.button_scroll_down {
        out.push(("scrolldown", v));
        out.push(("scrolldwn", v));
        out.push(("scrolldn", v));
    }
    out
}

/// Parses a buttons value into `button name (lowercase) -> value` (as given).
/// Also returns the requested keyboard layout, if any.
pub fn parse_mapping(setting: &Setting, mapping: &Value) -> Result<(Vec<(String, String)>, Option<String>)> {
    let available: Vec<String> = setting
        .buttons
        .iter()
        .flatten()
        .map(|b| b.name.to_lowercase())
        .collect();

    let mut entries: Vec<(String, String)> = Vec::new();
    match mapping {
        Value::String(s) if match_param_string(s).is_some() => {
            // is_buttons()
            let (name, params) = parse_param_string(s)?;
            if name != "buttons" {
                return Err(
                    "Buttons expression must looks like buttons(<BUTTON>=<VALUE>; <BUTTON_N>=<VALUE_N>)".into(),
                );
            }
            for (k, v) in &params {
                let (k, v) = (k.to_lowercase(), v.to_lowercase());
                if k == "layout" {
                    if !LAYOUTS.contains(&v.as_str()) {
                        return Err(format!("Unknown layout '{v}'"));
                    }
                } else if !available.contains(&k) {
                    return Err(format!("This mouse have no button named '{k}'"));
                }
            }
            entries.extend(params);
        }
        Value::String(s) if s.to_lowercase() == "default" => {}
        Value::Object(map) => {
            let buttons = map
                .get("buttons")
                .and_then(Value::as_object)
                .ok_or("KeyError: 'buttons'")?;
            for (k, v) in buttons {
                let v = v.as_str().ok_or_else(|| format!("Invalid button value {v}"))?;
                entries.push((k.clone(), v.to_string()));
            }
        }
        other => return Err(format!("Invalid input value '{}'", crate::core::pyval::py_str(other))),
    }

    // The "layout" key is looked up case-sensitively, like in rivalcfg
    let layout = entries.iter().find(|(k, _)| k == "layout").map(|(_, v)| v.to_lowercase());
    let mut result = Vec::new();
    for (k, v) in entries {
        let k = k.to_lowercase();
        if k == "layout" {
            continue;
        }
        if !available.contains(&k) {
            return Err(format!("Unknown button name '{k}'"));
        }
        result.push((k, v));
    }
    Ok((result, layout))
}

/// Resolves the effective value of each button (defaults applied), in profile order.
pub fn effective_mapping(setting: &Setting, mapping: &Value) -> Result<Vec<(String, String)>> {
    let (entries, _) = parse_mapping(setting, mapping)?;
    let overrides: HashMap<String, String> = entries.into_iter().collect();
    Ok(setting
        .buttons
        .iter()
        .flatten()
        .map(|b| {
            let key = b.name.to_lowercase();
            let value = match overrides.get(&key) {
                Some(v) if v.to_lowercase() != "default" => v.clone(),
                _ => b.default.clone(),
            };
            (b.name.clone(), value.to_lowercase())
        })
        .collect())
}

pub fn process_value(setting: &Setting, mapping: &Value) -> Result<Vec<i64>> {
    let (_, layout) = parse_mapping(setting, mapping)?;
    let buttons = setting.buttons.as_deref().ok_or("missing buttons")?;
    let field_length = setting.button_field_length.ok_or("missing button_field_length")?;
    let specials = special_actions(setting);
    let layouts = &catalog().layouts;

    if setting.button_keyboard.is_some() {
        let name = layout.unwrap_or_else(|| "qwerty".into());
        if !LAYOUTS.contains(&name.as_str()) {
            return Err(format!("Unsupported layout '{name}'"));
        }
    }

    let mut packet = vec![0i64; buttons.len() * field_length];
    let mut put = |offset: usize, v: i64| -> Result<()> {
        *packet.get_mut(offset).ok_or("list assignment index out of range")? = v;
        Ok(())
    };
    for (button, value) in buttons.iter().zip(effective_mapping(setting, mapping)?) {
        let value = value.1;
        if let Some(target) = buttons.iter().find(|b| b.name.to_lowercase() == value) {
            put(button.offset, target.id as i64)?;
        } else if let Some((_, code)) = specials.iter().find(|(n, _)| *n == value) {
            put(button.offset, *code as i64)?;
        } else if let Some(code) = layouts.multimedia.get(&value) {
            let mm = setting
                .button_multimedia
                .ok_or("an integer is required (got type NoneType)")?;
            put(button.offset, mm as i64)?;
            put(button.offset + 1, *code as i64)?;
        } else if let (Some(kb), Some(code)) = (setting.button_keyboard, layouts.qwerty.get(&value)) {
            put(button.offset, kb as i64)?;
            put(button.offset + 1, *code as i64)?;
        } else {
            return Err(format!("Unknown button, key or action '{value}'"));
        }
    }
    Ok(packet)
}
