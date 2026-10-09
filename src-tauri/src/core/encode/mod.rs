//! Port of `rivalcfg.handlers.*` and of the packet building done in
//! `rivalcfg.mouse.Mouse`. Verified byte-for-byte by `tests/golden.rs`.

mod buttons;
mod choice;
mod color;
mod gradient;
mod range;
mod range_choice;

use serde_json::Value;

use super::profile::{Command, Profile, Setting, REPORT_TYPE_FEATURE, REPORT_TYPE_OUTPUT};
use super::pyval::Result;

pub use buttons::{effective_mapping, special_actions};
pub use choice::ordered_keys as choice_keys;
pub use color::is_reactive_off;
pub use gradient::{parse_gradient_raw, parse_gradient_value, parse_rgbgradient_string};
pub use range::{parse_dpi_list, process_range};
pub use range_choice::normalize_xy;

/// A HID report ready to be sent (`data[0]` is the report id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub report_type: u8,
    pub data: Vec<u8>,
    pub readback_length: Option<usize>,
}

/// Encodes the value-dependent part of a setting command (`handler.process_value`).
pub fn encode_value(setting: &Setting, value: &Value, selected_preset: Option<i64>) -> Result<Vec<i64>> {
    match setting.value_type() {
        "" | "none" => Ok(vec![]),
        "choice" => choice::process_value(setting, value),
        "range" => range::process_value(setting, value),
        "multidpi_range" => range::process_multidpi(setting, value, selected_preset),
        "range_choice" => range_choice::process_value(setting, value),
        "multidpi_range_choice" => range_choice::process_multidpi(setting, value, selected_preset),
        "multidpi_range_choice_xy" => range_choice::process_multidpi_xy(setting, value, selected_preset),
        "rgbcolor" => color::process_rgbcolor(value),
        "reactive_rgbcolor" => color::process_reactive(value),
        "rgbgradient" => gradient::process_v1(setting, value),
        "rgbgradientv2" => gradient::process_v2(setting, value),
        "buttons" => buttons::process_value(setting, value),
        other => Err(format!("Unknown handler '{other}' for '{}' setting", setting.name)),
    }
}

/// Builds the full packet for a setting (`Mouse.set_<setting>(value)`).
pub fn setting_packet(setting: &Setting, value: &Value, selected_preset: Option<i64>) -> Result<Packet> {
    let data = encode_value(setting, value, selected_preset)?;
    let mut body: Vec<i64> = setting.command.iter().map(|b| *b as i64).collect();
    body.extend(data);
    if let Some(suffix) = &setting.command_suffix {
        body.extend(suffix.iter().map(|b| *b as i64));
    }
    Ok(Packet {
        report_type: setting.report_type,
        data: frame(setting.report_type, body, setting.packet_length.unwrap_or(0))?,
        readback_length: setting.readback_length.filter(|l| *l > 0),
    })
}

/// Builds a packet for a raw command (save, battery, firmware...).
pub fn command_packet(report_type: u8, command: &[u8], packet_length: usize) -> Result<Packet> {
    Ok(Packet {
        report_type,
        data: frame(report_type, command.iter().map(|b| *b as i64).collect(), packet_length)?,
        readback_length: None,
    })
}

pub fn save_packet(profile: &Profile) -> Result<Packet> {
    let cmd: &Command = profile
        .save_command
        .as_ref()
        .ok_or("This mouse does not provide any save command.")?;
    let mut packet = command_packet(cmd.report_type, &cmd.command, cmd.packet_length.unwrap_or(0))?;
    packet.readback_length = cmd.readback_length.filter(|l| *l > 0);
    Ok(packet)
}

/// `Mouse._hid_write`: prepend report id 0x00, pad to `packet_length`, check bytes.
fn frame(report_type: u8, body: Vec<i64>, packet_length: usize) -> Result<Vec<u8>> {
    if report_type != REPORT_TYPE_OUTPUT && report_type != REPORT_TYPE_FEATURE {
        return Err(format!("Invalid HID report type: {report_type:2x}"));
    }
    let mut out = Vec::with_capacity(1 + body.len().max(packet_length));
    out.push(0x00);
    for b in &body {
        if !(0..256).contains(b) {
            return Err("byte must be in range(0, 256)".into());
        }
        out.push(*b as u8);
    }
    if packet_length > body.len() {
        out.resize(1 + packet_length, 0x00);
    }
    Ok(out)
}
