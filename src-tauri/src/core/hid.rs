//! Device discovery and HID transport (`rivalcfg.usbhid`).
//!
//! Set `RIVALGUI_DRY=1` (or `RIVALGUI_DRY=1038:1838,1038:1702`) to simulate
//! plugged devices; packets are then logged to stderr instead of being sent.

use std::ffi::CString;

use hidapi::{HidApi, HidDevice};

use super::device::{Device, Transport};
use super::profile::{catalog, Profile, REPORT_TYPE_FEATURE};
use super::pyval::Result;

pub const STEELSERIES_VID: u16 = 0x1038;
const DRY_DEFAULT: &str = "1038:1838,1038:1824,1038:1724";

/// A supported device found on the system.
#[derive(Debug, Clone)]
pub struct Detected {
    pub key: String,
    pub profile: &'static Profile,
    /// Candidate HID paths for the control interface, best first
    paths: Vec<CString>,
}

pub fn device_key(vendor_id: u16, product_id: u16) -> String {
    format!("{vendor_id:04x}:{product_id:04x}")
}

pub struct HidBackend {
    api: Option<HidApi>,
    dry: Option<Vec<(u16, u16)>>,
    last_error: Option<String>,
}

impl HidBackend {
    pub fn new() -> Self {
        let dry = std::env::var("RIVALGUI_DRY").ok().map(|v| {
            let spec = if v.contains(':') { v } else { DRY_DEFAULT.to_string() };
            spec.split(',')
                .filter_map(|id| {
                    let (v, p) = id.trim().split_once(':')?;
                    Some((u16::from_str_radix(v, 16).ok()?, u16::from_str_radix(p, 16).ok()?))
                })
                .collect()
        });
        let (api, last_error) = if dry.is_some() {
            (None, None)
        } else {
            match HidApi::new() {
                Ok(api) => (Some(api), None),
                Err(e) => (None, Some(e.to_string())),
            }
        };
        Self { api, dry, last_error }
    }

    pub fn is_dry(&self) -> bool {
        self.dry.is_some()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Lists the supported devices currently plugged.
    pub fn scan(&mut self) -> Vec<Detected> {
        let cat = catalog();
        if let Some(dry) = &self.dry {
            return dry
                .iter()
                .filter_map(|(v, p)| {
                    Some(Detected { key: device_key(*v, *p), profile: cat.get(*v, *p)?, paths: vec![] })
                })
                .collect();
        }
        let Some(api) = self.api.as_mut() else {
            self.api = HidApi::new().ok();
            return vec![];
        };
        if let Err(e) = api.refresh_devices() {
            self.last_error = Some(e.to_string());
            return vec![];
        }

        let mut found: Vec<(Detected, Vec<(u16, CString)>)> = Vec::new();
        for info in api.device_list() {
            if info.vendor_id() != STEELSERIES_VID {
                continue;
            }
            let Some(profile) = cat.get(info.vendor_id(), info.product_id()) else {
                continue;
            };
            let key = device_key(info.vendor_id(), info.product_id());
            let index = match found.iter().position(|(d, _)| d.key == key) {
                Some(i) => i,
                None => {
                    found.push((Detected { key, profile, paths: vec![] }, vec![]));
                    found.len() - 1
                }
            };
            if info.interface_number() == profile.endpoint {
                found[index].1.push((info.usage_page(), info.path().to_owned()));
            }
        }
        found
            .into_iter()
            .map(|(mut d, mut paths)| {
                // Vendor-defined collections are the control ones; try them first
                paths.sort_by_key(|(page, _)| if *page >= 0xFF00 { 0 } else { 1 });
                d.paths = paths.into_iter().map(|(_, p)| p).collect();
                d
            })
            .collect()
    }

    /// Opens a detected device.
    pub fn open(&self, detected: &Detected) -> Result<Device> {
        if self.dry.is_some() {
            return Ok(Device::new(detected.profile, Box::new(DryTransport { key: detected.key.clone() })));
        }
        let api = self.api.as_ref().ok_or("HID API unavailable")?;
        if detected.paths.is_empty() {
            return Err(format!(
                "Requested device or endpoint not found: {}:{:02x}",
                detected.key, detected.profile.endpoint
            ));
        }
        let mut handles = Vec::new();
        let mut last_error = String::new();
        for path in &detected.paths {
            match api.open_path(path) {
                Ok(h) => handles.push(h),
                Err(e) => last_error = e.to_string(),
            }
        }
        if handles.is_empty() {
            return Err(permission_hint(last_error));
        }
        Ok(Device::new(detected.profile, Box::new(HidTransport { handles, current: 0 })))
    }
}

impl Default for HidBackend {
    fn default() -> Self {
        Self::new()
    }
}

fn permission_hint(error: String) -> String {
    if cfg!(target_os = "linux") {
        crate::i18n::coded("open_failed_udev", error)
    } else {
        crate::i18n::coded("open_failed", error)
    }
}

/// Real transport. Holds every candidate collection and sticks to the first one that accepts writes.
struct HidTransport {
    handles: Vec<HidDevice>,
    current: usize,
}

impl HidTransport {
    fn write_one(handle: &HidDevice, report_type: u8, data: &[u8]) -> std::result::Result<(), String> {
        if report_type == REPORT_TYPE_FEATURE {
            handle.send_feature_report(data).map_err(|e| e.to_string())
        } else {
            handle.write(data).map(|_| ()).map_err(|e| e.to_string())
        }
    }
}

impl Transport for HidTransport {
    fn write(&mut self, report_type: u8, data: &[u8]) -> Result<()> {
        let mut error = String::new();
        for attempt in 0..self.handles.len() {
            let index = (self.current + attempt) % self.handles.len();
            match Self::write_one(&self.handles[index], report_type, data) {
                Ok(()) => {
                    self.current = index;
                    return Ok(());
                }
                Err(e) => error = e,
            }
        }
        Err(format!("HID write failed: {error}"))
    }

    fn read(&mut self, length: usize, timeout_ms: i32) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; length];
        let n = self.handles[self.current]
            .read_timeout(&mut buf, timeout_ms)
            .map_err(|e| format!("HID read failed: {e}"))?;
        buf.truncate(n);
        Ok(buf)
    }
}

/// Simulated device (RIVALGUI_DRY).
struct DryTransport {
    key: String,
}

impl Transport for DryTransport {
    fn write(&mut self, report_type: u8, data: &[u8]) -> Result<()> {
        let hex: Vec<String> = data.iter().map(|b| format!("{b:02x}")).collect();
        eprintln!("[dry {}] {:02x} {}", self.key, report_type, hex.join(" "));
        Ok(())
    }

    fn read(&mut self, length: usize, _timeout_ms: i32) -> Result<Vec<u8>> {
        // Plausible battery answer for both formats: 70% charging / 80%
        let mut data = vec![0x50, 0x80 | 15, 0x00];
        data.resize(length, 0);
        Ok(data)
    }
}
