//! Persistent configuration: app preferences and named setting profiles per device.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::i18n::coded;

pub const DEFAULT_PROFILE: &str = "Default";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// Re-apply the active profile when a mouse is plugged in or wakes up
    pub auto_apply: bool,
    pub notify_low_battery: bool,
    pub low_battery_threshold: u8,
    /// UI / tray language: "auto" (system), "en" or "pl"
    pub language: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self { auto_apply: true, notify_low_battery: true, low_battery_threshold: 15, language: "auto".into() }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NamedProfile {
    pub name: String,
    /// Values explicitly set by the user (rivalcfg value syntax)
    #[serde(default)]
    pub values: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub active: String,
    pub profiles: Vec<NamedProfile>,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            active: DEFAULT_PROFILE.into(),
            profiles: vec![NamedProfile { name: DEFAULT_PROFILE.into(), values: Map::new() }],
        }
    }
}

impl DeviceConfig {
    pub fn active_profile(&self) -> Option<&NamedProfile> {
        self.profiles.iter().find(|p| p.name == self.active)
    }

    pub fn active_profile_mut(&mut self) -> &mut NamedProfile {
        if !self.profiles.iter().any(|p| p.name == self.active) {
            self.active = self.profiles.first().map(|p| p.name.clone()).unwrap_or_default();
            if self.profiles.is_empty() {
                *self = Self::default();
            }
        }
        let active = self.active.clone();
        self.profiles.iter_mut().find(|p| p.name == active).unwrap()
    }

    pub fn profile_names(&self) -> Vec<String> {
        self.profiles.iter().map(|p| p.name.clone()).collect()
    }

    pub fn create(&mut self, name: &str, copy_from: Option<&str>) -> Result<(), String> {
        let name = validate_name(name)?;
        if self.profiles.iter().any(|p| p.name == name) {
            return Err(coded("profile_exists", name));
        }
        let values = copy_from
            .and_then(|src| self.profiles.iter().find(|p| p.name == src))
            .map(|p| p.values.clone())
            .unwrap_or_default();
        self.profiles.push(NamedProfile { name, values });
        Ok(())
    }

    pub fn rename(&mut self, old: &str, new: &str) -> Result<(), String> {
        let new = validate_name(new)?;
        if old != new && self.profiles.iter().any(|p| p.name == new) {
            return Err(coded("profile_exists", new));
        }
        let profile = self
            .profiles
            .iter_mut()
            .find(|p| p.name == old)
            .ok_or_else(|| coded("profile_missing", old))?;
        profile.name = new.clone();
        if self.active == old {
            self.active = new;
        }
        Ok(())
    }

    pub fn delete(&mut self, name: &str) -> Result<(), String> {
        if self.profiles.len() <= 1 {
            return Err(coded("last_profile", ""));
        }
        self.profiles.retain(|p| p.name != name);
        if self.active == name {
            self.active = self.profiles[0].name.clone();
        }
        Ok(())
    }

    pub fn activate(&mut self, name: &str) -> Result<(), String> {
        if !self.profiles.iter().any(|p| p.name == name) {
            return Err(coded("profile_missing", name));
        }
        self.active = name.into();
        Ok(())
    }
}

fn validate_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 40 {
        return Err(coded("profile_name", ""));
    }
    Ok(name.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "config_version")]
    pub version: u32,
    #[serde(default)]
    pub prefs: Prefs,
    /// Keyed by "vvvv:pppp"
    #[serde(default)]
    pub devices: BTreeMap<String, DeviceConfig>,
}

fn config_version() -> u32 {
    1
}

impl Default for Config {
    fn default() -> Self {
        Self { version: config_version(), prefs: Prefs::default(), devices: BTreeMap::new() }
    }
}

pub struct Store {
    path: Option<PathBuf>,
    pub config: Config,
}

impl Store {
    pub fn config_path() -> Option<PathBuf> {
        if let Ok(dir) = std::env::var("RIVALGUI_CONFIG_DIR") {
            return Some(PathBuf::from(dir).join("config.json"));
        }
        dirs::config_dir().map(|d| d.join("rivalgui").join("config.json"))
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let config = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| match serde_json::from_str(&text) {
                Ok(c) => Some(c),
                Err(e) => {
                    eprintln!("rivalgui: ignoring invalid config: {e}");
                    None
                }
            })
            .unwrap_or_default();
        Self { path, config }
    }

    /// Writes the config atomically (temp file + rename).
    pub fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else { return Ok(()) };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(&self.config).map_err(|e| e.to_string())?;
        std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    }

    pub fn device(&mut self, key: &str) -> &mut DeviceConfig {
        self.config.devices.entry(key.to_string()).or_default()
    }

    pub fn device_ref(&self, key: &str) -> Option<&DeviceConfig> {
        self.config.devices.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_management() {
        let mut d = DeviceConfig::default();
        d.active_profile_mut().values.insert("color".into(), "red".into());
        d.create("Gaming", Some("Default")).unwrap();
        assert!(d.create("gaming ", None).is_ok());
        assert!(d.create("Gaming", None).is_err());
        d.activate("Gaming").unwrap();
        assert_eq!(d.active_profile().unwrap().values["color"], "red");
        d.rename("Gaming", "FPS").unwrap();
        assert_eq!(d.active, "FPS");
        d.delete("FPS").unwrap();
        assert_eq!(d.active, "Default");
        d.delete("gaming").unwrap();
        assert!(d.delete("Default").is_err());
    }
}
