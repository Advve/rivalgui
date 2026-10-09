//! High level device access (`rivalcfg.mouse.Mouse`).

use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use super::encode::{self, Packet};
use super::profile::{BatteryFormat, Profile, Setting};
use super::pyval::Result;

/// Delay between two commands so the device is not overloaded (Kinzu v2 & co).
pub const COMMAND_DELAY: Duration = Duration::from_millis(50);
const READ_TIMEOUT_MS: i32 = 200;

/// Low level I/O used by [`Device`].
pub trait Transport: Send {
    /// Sends a report (`data[0]` is the report id).
    fn write(&mut self, report_type: u8, data: &[u8]) -> Result<()>;
    /// Reads up to `length` bytes (empty on timeout).
    fn read(&mut self, length: usize, timeout_ms: i32) -> Result<Vec<u8>>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Battery {
    pub level: Option<u8>,
    pub charging: Option<bool>,
}

impl Battery {
    pub fn is_known(&self) -> bool {
        self.level.is_some()
    }
}

/// Decodes a battery response (the lambdas of rivalcfg profiles).
pub fn parse_battery(format: BatteryFormat, data: &[u8]) -> Battery {
    let (level, charging) = match format {
        BatteryFormat::Aerox => match data.get(1) {
            Some(b) => (Some(((b & 0x7F) as i64 - 1) * 5), Some(b & 0x80 != 0)),
            None => (None, None),
        },
        BatteryFormat::Rival => (data.first().map(|b| *b as i64), data.get(2).map(|b| *b != 0)),
    };
    match level {
        Some(l) if (0..=100).contains(&l) => Battery { level: Some(l as u8), charging },
        _ => Battery::default(),
    }
}

pub struct Device {
    pub profile: &'static Profile,
    transport: Box<dyn Transport>,
}

impl Device {
    pub fn new(profile: &'static Profile, transport: Box<dyn Transport>) -> Self {
        Self { profile, transport }
    }

    fn send(&mut self, packet: &Packet) -> Result<Option<Vec<u8>>> {
        self.transport.write(packet.report_type, &packet.data)?;
        std::thread::sleep(COMMAND_DELAY);
        match packet.readback_length {
            Some(length) => Ok(Some(self.transport.read(length, READ_TIMEOUT_MS)?)),
            None => Ok(None),
        }
    }

    pub fn set(&mut self, setting: &Setting, value: &Value, selected_preset: Option<i64>) -> Result<()> {
        let packet = encode::setting_packet(setting, value, selected_preset)?;
        self.send(&packet).map(|_| ())
    }

    /// Persists the current configuration in the mouse memory.
    pub fn save(&mut self) -> Result<()> {
        let packet = encode::save_packet(self.profile)?;
        self.send(&packet).map(|_| ())
    }

    pub fn has_battery(&self) -> bool {
        self.profile.battery_level.is_some()
    }

    pub fn battery(&mut self) -> Result<Battery> {
        let Some(cmd) = self.profile.battery_level.as_ref() else {
            return Ok(Battery::default());
        };
        let packet = encode::command_packet(cmd.report_type, &cmd.command, 0)?;
        self.send(&packet)?;
        let data = self.transport.read(cmd.response_length, READ_TIMEOUT_MS)?;
        Ok(parse_battery(cmd.format, &data))
    }

    pub fn firmware_version(&mut self) -> Result<Option<String>> {
        let Some(cmd) = self.profile.firmware_version.as_ref() else {
            return Ok(None);
        };
        let packet = encode::command_packet(cmd.report_type, &cmd.command, 0)?;
        self.send(&packet)?;
        let data = self.transport.read(cmd.response_length, READ_TIMEOUT_MS)?;
        Ok((!data.is_empty()).then(|| {
            data.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(".")
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_formats() {
        let b = parse_battery(BatteryFormat::Aerox, &[0x00, 0x85]);
        assert_eq!(b, Battery { level: Some(20), charging: Some(true) });
        let b = parse_battery(BatteryFormat::Rival, &[42, 0, 0]);
        assert_eq!(b, Battery { level: Some(42), charging: Some(false) });
        assert_eq!(parse_battery(BatteryFormat::Aerox, &[]), Battery::default());
        assert_eq!(parse_battery(BatteryFormat::Aerox, &[0, 0x00]), Battery::default());
        assert_eq!(parse_battery(BatteryFormat::Rival, &[200, 0, 1]), Battery::default());
    }
}
