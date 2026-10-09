//! Tauri IPC commands used by the UI.

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::core::profile::catalog;
use crate::core::view::SettingView;
use crate::i18n::coded;
use crate::service::state::{AppState, DeviceDetail, DeviceStatus};
use crate::service::store::{Prefs, Store};
use crate::service::watcher::publish;

type CmdResult<T> = Result<T, String>;

/// Runs device I/O off the main thread.
async fn blocking<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&AppHandle) -> CmdResult<T> + Send + 'static,
) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(move || f(&app))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    platform: &'static str,
    dry_run: bool,
    hid_error: Option<String>,
    config_path: Option<String>,
    supported_devices: usize,
    qwerty_keys: Vec<String>,
    multimedia_keys: Vec<String>,
    prefs: Prefs,
    autostart: bool,
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> AppInfo {
    let state = app.state::<AppState>();
    let cat = catalog();
    let (dry_run, hid_error) = {
        let hid = state.hid.lock().unwrap();
        (hid.is_dry(), hid.last_error().map(String::from))
    };
    let prefs = state.store.lock().unwrap().config.prefs.clone();
    AppInfo {
        version: app.package_info().version.to_string(),
        platform: std::env::consts::OS,
        dry_run,
        hid_error,
        config_path: Store::config_path().map(|p| p.display().to_string()),
        supported_devices: cat.profiles.len(),
        qwerty_keys: cat.layouts.qwerty_keys.clone(),
        multimedia_keys: cat.layouts.multimedia_keys.clone(),
        prefs,
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
    }
}

#[tauri::command]
pub fn list_devices(app: AppHandle) -> Vec<DeviceStatus> {
    app.state::<AppState>().statuses()
}

#[tauri::command]
pub async fn get_device(app: AppHandle, key: String) -> CmdResult<DeviceDetail> {
    blocking(app, move |app| app.state::<AppState>().detail(&key)).await
}

#[tauri::command]
pub async fn set_setting(app: AppHandle, key: String, name: String, value: Value) -> CmdResult<SettingView> {
    blocking(app, move |app| app.state::<AppState>().set_value(&key, &name, value)).await
}

#[tauri::command]
pub async fn reset_setting(app: AppHandle, key: String, name: String) -> CmdResult<SettingView> {
    blocking(app, move |app| app.state::<AppState>().reset_value(&key, &name)).await
}

#[tauri::command]
pub async fn run_action(app: AppHandle, key: String, name: String) -> CmdResult<()> {
    blocking(app, move |app| {
        let state = app.state::<AppState>();
        if !state.status(&key).is_some_and(|s| s.online) {
            return Err(coded("asleep", ""));
        }
        state.set_value(&key, &name, Value::Null).map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn save_to_device(app: AppHandle, key: String) -> CmdResult<()> {
    blocking(app, move |app| app.state::<AppState>().with_device(&key, |d| d.save())).await
}

#[tauri::command]
pub async fn apply_profile(app: AppHandle, key: String) -> CmdResult<Vec<String>> {
    blocking(app, move |app| app.state::<AppState>().apply_profile(&key, true)).await
}

/// Clears every value of the active profile and sends the defaults.
#[tauri::command]
pub async fn reset_profile(app: AppHandle, key: String) -> CmdResult<DeviceDetail> {
    blocking(app, move |app| {
        let state = app.state::<AppState>();
        {
            let mut store = state.store.lock().unwrap();
            store.device(&key).active_profile_mut().values.clear();
            store.save()?;
        }
        state.apply_profile(&key, true)?;
        state.detail(&key)
    })
    .await
}

pub fn activate_profile_impl(app: &AppHandle, key: &str, name: &str) -> CmdResult<DeviceDetail> {
    let state = app.state::<AppState>();
    {
        let mut store = state.store.lock().unwrap();
        store.device(key).activate(name)?;
        store.save()?;
    }
    state.sync_active_profile(key);
    let result = if state.status(key).is_some_and(|s| s.online) {
        state.apply_profile(key, true).map(|_| ())
    } else {
        Ok(())
    };
    publish(app);
    result?;
    state.detail(key)
}

#[tauri::command]
pub async fn activate_profile(app: AppHandle, key: String, name: String) -> CmdResult<DeviceDetail> {
    blocking(app, move |app| activate_profile_impl(app, &key, &name)).await
}

#[tauri::command]
pub async fn create_profile(app: AppHandle, key: String, name: String, copy_from: Option<String>) -> CmdResult<DeviceDetail> {
    blocking(app, move |app| {
        {
            let state = app.state::<AppState>();
            let mut store = state.store.lock().unwrap();
            store.device(&key).create(&name, copy_from.as_deref())?;
            store.save()?;
        }
        activate_profile_impl(app, &key, name.trim())
    })
    .await
}

#[tauri::command]
pub async fn rename_profile(app: AppHandle, key: String, old: String, new: String) -> CmdResult<DeviceDetail> {
    blocking(app, move |app| {
        let state = app.state::<AppState>();
        {
            let mut store = state.store.lock().unwrap();
            store.device(&key).rename(&old, &new)?;
            store.save()?;
        }
        state.sync_active_profile(&key);
        publish(app);
        state.detail(&key)
    })
    .await
}

#[tauri::command]
pub async fn delete_profile(app: AppHandle, key: String, name: String) -> CmdResult<DeviceDetail> {
    blocking(app, move |app| {
        let state = app.state::<AppState>();
        let (was_active, active) = {
            let mut store = state.store.lock().unwrap();
            let config = store.device(&key);
            let was_active = config.active == name;
            config.delete(&name)?;
            let active = config.active.clone();
            store.save()?;
            (was_active, active)
        };
        if was_active {
            return activate_profile_impl(app, &key, &active);
        }
        publish(app);
        state.detail(&key)
    })
    .await
}

#[tauri::command]
pub fn set_prefs(app: AppHandle, prefs: Prefs) -> CmdResult<Prefs> {
    let state = app.state::<AppState>();
    let mut store = state.store.lock().unwrap();
    store.config.prefs = prefs;
    store.save()?;
    let prefs = store.config.prefs.clone();
    drop(store);
    publish(&app);
    Ok(prefs)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> CmdResult<bool> {
    let launcher = app.autolaunch();
    if enabled {
        launcher.enable().map_err(|e| e.to_string())?;
    } else {
        launcher.disable().map_err(|e| e.to_string())?;
    }
    publish(&app);
    launcher.is_enabled().map_err(|e| e.to_string())
}

/// udev rules giving users access to supported devices (`rivalcfg --update-udev`),
/// generated by tools/export_profiles.py and also shipped in the .deb/.rpm.
const UDEV_RULES: &str = include_str!("../resources/99-steelseries-rival.rules");

#[tauri::command]
pub async fn install_udev_rules() -> CmdResult<()> {
    if !cfg!(target_os = "linux") {
        return Err(coded("udev_linux_only", ""));
    }
    tauri::async_runtime::spawn_blocking(|| {
        let tmp = std::env::temp_dir().join("99-steelseries-rival.rules");
        std::fs::write(&tmp, UDEV_RULES).map_err(|e| e.to_string())?;
        let status = std::process::Command::new("pkexec")
            .args([
                "sh",
                "-c",
                "install -m 644 \"$0\" /etc/udev/rules.d/99-steelseries-rival.rules && udevadm control --reload-rules && udevadm trigger",
            ])
            .arg(&tmp)
            .status()
            .map_err(|e| coded("pkexec", e))?;
        let _ = std::fs::remove_file(&tmp);
        if status.success() {
            Ok(())
        } else {
            Err(coded("udev_failed", ""))
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
