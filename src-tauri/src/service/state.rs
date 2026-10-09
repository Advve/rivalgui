//! Shared application state and device operations.

use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use serde_json::Value;

use crate::core::device::{Battery, Device};
use crate::core::encode;
use crate::core::hid::{Detected, HidBackend};
use crate::core::profile::Profile;
use crate::core::view::{display_label, setting_view, SettingView};
use crate::i18n::{coded, Lang};

use super::store::Store;

/// What the UI and the tray know about a plugged device.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceStatus {
    pub key: String,
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub wireless: bool,
    /// Wireless: the mouse answers (not asleep / turned off)
    pub online: bool,
    pub battery: Option<Battery>,
    pub firmware: Option<String>,
    pub can_save: bool,
    pub active_profile: String,
    pub error: Option<String>,
}

pub struct Tracked {
    pub detected: Detected,
    pub status: DeviceStatus,
    pub last_poll: Option<Instant>,
    pub low_battery_notified: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceDetail {
    pub status: DeviceStatus,
    pub settings: Vec<SettingView>,
    pub profiles: Vec<String>,
    pub active: String,
}

pub struct AppState {
    pub hid: Mutex<HidBackend>,
    pub store: Mutex<Store>,
    pub devices: Mutex<Vec<Tracked>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            hid: Mutex::new(HidBackend::new()),
            store: Mutex::new(Store::load()),
            devices: Mutex::new(Vec::new()),
        }
    }

    /// Language of the tray and notifications (from the preferences).
    pub fn lang(&self) -> Lang {
        Lang::from_pref(&self.store.lock().unwrap().config.prefs.language)
    }

    pub fn statuses(&self) -> Vec<DeviceStatus> {
        self.devices.lock().unwrap().iter().map(|t| t.status.clone()).collect()
    }

    pub fn status(&self, key: &str) -> Option<DeviceStatus> {
        self.devices.lock().unwrap().iter().find(|t| t.status.key == key).map(|t| t.status.clone())
    }

    fn detected(&self, key: &str) -> Result<Detected, String> {
        self.devices
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.status.key == key)
            .map(|t| t.detected.clone())
            .ok_or_else(|| coded("not_connected", ""))
    }

    pub fn update_status(&self, key: &str, f: impl FnOnce(&mut DeviceStatus)) {
        if let Some(t) = self.devices.lock().unwrap().iter_mut().find(|t| t.status.key == key) {
            f(&mut t.status);
        }
    }

    /// Opens the device and runs `f` while holding the HID lock (serializes all I/O).
    pub fn with_device<T>(&self, key: &str, f: impl FnOnce(&mut Device) -> Result<T, String>) -> Result<T, String> {
        let detected = self.detected(key)?;
        let hid = self.hid.lock().unwrap();
        let result = hid.open(&detected).and_then(|mut device| f(&mut device));
        drop(hid);
        let error = result.as_ref().err().cloned();
        self.update_status(key, |s| s.error = error);
        result
    }

    pub fn profile_of(&self, key: &str) -> Result<&'static Profile, String> {
        Ok(self.detected(key)?.profile)
    }

    pub fn detail(&self, key: &str) -> Result<DeviceDetail, String> {
        let status = self.status(key).ok_or_else(|| coded("not_connected", ""))?;
        let profile = self.profile_of(key)?;
        let mut store = self.store.lock().unwrap();
        let config = store.device(key);
        let values = config.active_profile_mut().values.clone();
        Ok(DeviceDetail {
            settings: profile.settings.iter().map(|s| setting_view(s, values.get(&s.name))).collect(),
            profiles: config.profile_names(),
            active: config.active.clone(),
            status,
        })
    }

    /// Applies the active profile. With `full`, unset settings are reset to
    /// their defaults; otherwise only explicitly set values are sent (so a
    /// mouse configured elsewhere is not overwritten on first plug-in).
    pub fn apply_profile(&self, key: &str, full: bool) -> Result<Vec<String>, String> {
        let profile = self.profile_of(key)?;
        let values = {
            let mut store = self.store.lock().unwrap();
            store.device(key).active_profile_mut().values.clone()
        };
        let plan: Vec<(&crate::core::profile::Setting, Value)> = profile
            .settings
            .iter()
            .filter(|s| !s.is_action())
            .filter_map(|s| match values.get(&s.name) {
                Some(v) => Some((s, v.clone())),
                None if full => s.default.clone().map(|d| (s, d)),
                None => None,
            })
            .collect();
        if plan.is_empty() {
            return Ok(vec![]);
        }
        self.with_device(key, |device| {
            let mut errors = Vec::new();
            for (setting, value) in &plan {
                if let Err(e) = device.set(setting, value, None) {
                    errors.push(format!("{}: {e}", display_label(setting)));
                }
            }
            Ok(errors)
        })
    }

    /// Validates, applies (when the mouse is reachable) and stores a value.
    pub fn set_value(&self, key: &str, name: &str, value: Value) -> Result<SettingView, String> {
        let profile = self.profile_of(key)?;
        let setting = profile.setting(name).ok_or_else(|| format!("Unknown setting '{name}'"))?;
        // Reject invalid values before touching the device or the config
        encode::setting_packet(setting, &value, None)?;
        let online = self.status(key).map(|s| s.online).unwrap_or(false);
        if online {
            self.with_device(key, |device| device.set(setting, &value, None))?;
        }
        if !setting.is_action() {
            let mut store = self.store.lock().unwrap();
            store.device(key).active_profile_mut().values.insert(name.to_string(), value.clone());
            store.save()?;
        }
        Ok(setting_view(setting, Some(&value)))
    }

    /// Forgets a stored value and sends the default to the mouse.
    pub fn reset_value(&self, key: &str, name: &str) -> Result<SettingView, String> {
        let profile = self.profile_of(key)?;
        let setting = profile.setting(name).ok_or_else(|| format!("Unknown setting '{name}'"))?;
        {
            let mut store = self.store.lock().unwrap();
            store.device(key).active_profile_mut().values.remove(name);
            store.save()?;
        }
        if let Some(default) = &setting.default {
            if self.status(key).is_some_and(|s| s.online) {
                self.with_device(key, |device| device.set(setting, default, None))?;
            }
        }
        Ok(setting_view(setting, None))
    }

    pub fn active_profile_name(&self, key: &str) -> String {
        let store = self.store.lock().unwrap();
        store
            .device_ref(key)
            .map(|c| c.active.clone())
            .unwrap_or_else(|| super::store::DEFAULT_PROFILE.into())
    }

    pub fn sync_active_profile(&self, key: &str) {
        let name = self.active_profile_name(key);
        self.update_status(key, |s| s.active_profile = name);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
