//! Device profiles exported from rivalcfg (`tools/export_profiles.py`).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const REPORT_TYPE_OUTPUT: u8 = 0x02;
pub const REPORT_TYPE_FEATURE: u8 = 0x03;

static CATALOG: OnceLock<Catalog> = OnceLock::new();

/// Returns the catalog of all supported devices (parsed once).
pub fn catalog() -> &'static Catalog {
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../../resources/profiles.json"))
            .expect("embedded profiles.json is invalid")
    })
}

#[derive(Debug, Deserialize)]
pub struct Catalog {
    pub rivalcfg_version: String,
    pub profiles: Vec<Profile>,
    pub layouts: Layouts,
}

impl Catalog {
    pub fn get(&self, vendor_id: u16, product_id: u16) -> Option<&Profile> {
        self.profiles
            .iter()
            .find(|p| p.vendor_id == vendor_id && p.product_id == product_id)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Layouts {
    /// Lowercase key name or alias -> keycode
    pub qwerty: HashMap<String, u8>,
    pub multimedia: HashMap<String, u8>,
    /// Canonical key names, in source order (for UI pickers)
    pub qwerty_keys: Vec<String>,
    pub multimedia_keys: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Profile {
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub endpoint: i32,
    pub settings: Vec<Setting>,
    #[serde(default)]
    pub save_command: Option<Command>,
    #[serde(default)]
    pub firmware_version: Option<QueryCommand>,
    #[serde(default)]
    pub battery_level: Option<BatteryCommand>,
}

impl Profile {
    pub fn setting(&self, name: &str) -> Option<&Setting> {
        self.settings.iter().find(|s| s.name == name)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Command {
    pub report_type: u8,
    pub command: Vec<u8>,
    #[serde(default)]
    pub packet_length: Option<usize>,
    #[serde(default)]
    pub readback_length: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueryCommand {
    pub report_type: u8,
    pub command: Vec<u8>,
    pub response_length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BatteryFormat {
    /// level = ((d[1] & 0x7F) - 1) * 5, charging = d[1] & 0x80
    Aerox,
    /// level = d[0], charging = d[2]
    Rival,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BatteryCommand {
    pub report_type: u8,
    pub command: Vec<u8>,
    pub response_length: usize,
    pub format: BatteryFormat,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Choice {
    pub key: Value,
    pub value: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ButtonDef {
    pub name: String,
    pub id: u8,
    pub offset: usize,
    pub default: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GradientHeader {
    pub header_length: usize,
    pub led_id_offsets: Vec<usize>,
    pub duration_offset: usize,
    pub duration_length: usize,
    pub repeat_offset: usize,
    pub triggers_offset: usize,
    pub color_count_offset: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GradientV2Header {
    pub color_field_length: usize,
    pub duration_length: usize,
    pub maxgradient: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Setting {
    pub name: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub description: String,
    pub report_type: u8,
    pub command: Vec<u8>,
    #[serde(default)]
    pub command_suffix: Option<Vec<u8>>,
    #[serde(default)]
    pub packet_length: Option<usize>,
    #[serde(default)]
    pub readback_length: Option<usize>,
    #[serde(default)]
    pub value_type: Option<String>,
    #[serde(default)]
    pub default: Option<Value>,

    // choice
    #[serde(default)]
    pub choices: Option<Vec<Choice>>,
    // range / multidpi_range
    #[serde(default)]
    pub input_range: Option<[i64; 3]>,
    #[serde(default)]
    pub output_range: Option<[f64; 3]>,
    /// range_choice handlers: (input DPI, device value) table of the sensor
    #[serde(default)]
    pub output_choices: Option<Vec<(i64, i64)>>,
    /// multidpi_range_choice_xy: "xyxy" or "xxyy"
    #[serde(default)]
    pub xy_mapping: Option<String>,
    #[serde(default)]
    pub range_length_byte: Option<usize>,
    #[serde(default)]
    pub dpi_length_byte: Option<usize>,
    #[serde(default)]
    pub first_preset: Option<i64>,
    #[serde(default)]
    pub count_mode: Option<String>,
    #[serde(default)]
    pub max_preset_count: Option<usize>,
    // rgbgradient / rgbgradientv2
    #[serde(default)]
    pub rgbgradient_header: Option<GradientHeader>,
    #[serde(default)]
    pub rgbgradientv2_header: Option<GradientV2Header>,
    #[serde(default)]
    pub led_id: Option<u8>,
    // buttons
    #[serde(default)]
    pub buttons: Option<Vec<ButtonDef>>,
    #[serde(default)]
    pub button_field_length: Option<usize>,
    #[serde(default)]
    pub button_disable: Option<u8>,
    #[serde(default)]
    pub button_keyboard: Option<u8>,
    #[serde(default)]
    pub button_multimedia: Option<u8>,
    #[serde(default)]
    pub button_dpi_switch: Option<u8>,
    #[serde(default)]
    pub button_scroll_up: Option<u8>,
    #[serde(default)]
    pub button_scroll_down: Option<u8>,
}

impl Setting {
    pub fn value_type(&self) -> &str {
        self.value_type.as_deref().unwrap_or("")
    }

    /// Settings without a value (e.g. "enable rainbow effect") are one-shot actions.
    pub fn is_action(&self) -> bool {
        matches!(self.value_type(), "none" | "")
    }
}
