//! UI-friendly description of settings and values.

use serde::Serialize;
use serde_json::Value;

use super::encode::{self};
use super::profile::Setting;
use super::pyval::{color_from_value, color_to_hex, py_int, py_str};

#[derive(Debug, Clone, Serialize)]
pub struct StopView {
    pub pos: i64,
    pub color: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ButtonView {
    pub button: String,
    pub value: String,
}

/// A stored value, decoded for the UI controls.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiValue {
    Choice { value: String },
    Range { value: i64 },
    Dpi { values: Vec<i64> },
    /// Separate X / Y DPI per preset
    DpiXy { values: Vec<[i64; 2]> },
    Color { color: String },
    Reactive { color: Option<String> },
    Gradient { is_gradient: bool, duration: i64, stops: Vec<StopView> },
    Buttons { mapping: Vec<ButtonView> },
    Action,
    Invalid { error: String },
}

/// Which targets a button can be mapped to on this device.
#[derive(Debug, Clone, Serialize)]
pub struct ButtonTargets {
    pub buttons: Vec<String>,
    pub specials: Vec<String>,
    pub keyboard: bool,
    pub multimedia: bool,
    /// Button name -> factory default action (lowercase)
    pub defaults: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingView {
    pub name: String,
    pub label: String,
    pub description: String,
    pub value_type: String,
    pub group: &'static str,
    pub value: Value,
    /// The value is stored in the profile (otherwise it is the default)
    pub is_set: bool,
    pub default: Option<Value>,
    pub ui: UiValue,
    pub choices: Option<Vec<String>>,
    pub range: Option<[i64; 3]>,
    pub max_presets: Option<usize>,
    pub max_stops: Option<usize>,
    pub button_targets: Option<ButtonTargets>,
}

pub fn group_of(name: &str) -> &'static str {
    if name.starts_with("sensitivity") || name == "polling_rate" {
        "performance"
    } else if name.starts_with("btn") || name.starts_with("buttons") {
        "buttons"
    } else if name.ends_with("_timer") {
        "power"
    } else {
        "lighting"
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The setting label as displayed (capitalized, rivalcfg typo fixed). Also the
/// key the UI uses to translate it.
pub fn display_label(setting: &Setting) -> String {
    capitalize(&setting.label).replace("Sensibility", "Sensitivity")
}

pub fn ui_value(setting: &Setting, value: &Value) -> UiValue {
    let result = || -> Result<UiValue, String> {
        Ok(match setting.value_type() {
            "choice" => UiValue::Choice { value: py_str(value) },
            "range" | "range_choice" => UiValue::Range { value: py_int(value)? },
            "multidpi_range" | "multidpi_range_choice" => UiValue::Dpi { values: encode::parse_dpi_list(value)? },
            "multidpi_range_choice_xy" => UiValue::DpiXy { values: encode::normalize_xy(value)? },
            "rgbcolor" => UiValue::Color { color: color_to_hex(color_from_value(value)?) },
            "reactive_rgbcolor" => UiValue::Reactive {
                color: if encode::is_reactive_off(value) {
                    None
                } else {
                    Some(color_to_hex(color_from_value(value)?))
                },
            },
            "rgbgradient" | "rgbgradientv2" => {
                let g = encode::parse_gradient_raw(value)?;
                UiValue::Gradient {
                    is_gradient: g.is_gradient,
                    duration: g.duration,
                    stops: g
                        .stops
                        .into_iter()
                        .map(|s| StopView { pos: s.pos, color: color_to_hex(s.color) })
                        .collect(),
                }
            }
            "buttons" => UiValue::Buttons {
                mapping: encode::effective_mapping(setting, value)?
                    .into_iter()
                    .map(|(button, value)| ButtonView { button, value })
                    .collect(),
            },
            _ => UiValue::Action,
        })
    };
    result().unwrap_or_else(|error| UiValue::Invalid { error })
}

pub fn setting_view(setting: &Setting, value: Option<&Value>) -> SettingView {
    let is_set = value.is_some();
    let value = value
        .cloned()
        .or_else(|| setting.default.clone())
        .unwrap_or(Value::Null);
    let vt = setting.value_type();
    SettingView {
        name: setting.name.clone(),
        label: display_label(setting),
        description: setting.description.clone(),
        value_type: if vt.is_empty() { "none".into() } else { vt.into() },
        group: group_of(&setting.name),
        ui: ui_value(setting, &value),
        value,
        is_set,
        default: setting.default.clone(),
        choices: (vt == "choice").then(|| encode::choice_keys(setting).iter().map(py_str).collect()),
        range: setting.input_range,
        max_presets: setting.max_preset_count,
        max_stops: match vt {
            "rgbgradient" => Some(14),
            "rgbgradientv2" => setting.rgbgradientv2_header.as_ref().map(|h| h.maxgradient),
            _ => None,
        },
        button_targets: (vt == "buttons").then(|| ButtonTargets {
            buttons: setting.buttons.iter().flatten().map(|b| b.name.clone()).collect(),
            specials: encode::special_actions(setting)
                .iter()
                .filter(|(n, _)| matches!(*n, "disabled" | "dpi" | "scrollup" | "scrolldown"))
                .map(|(n, _)| n.to_string())
                .collect(),
            keyboard: setting.button_keyboard.is_some(),
            multimedia: setting.button_multimedia.is_some(),
            defaults: setting
                .buttons
                .iter()
                .flatten()
                .map(|b| (b.name.clone(), b.default.to_lowercase()))
                .collect(),
        }),
    }
}
