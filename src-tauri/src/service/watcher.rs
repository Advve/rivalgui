//! Background thread: device hot-plug detection, wake-up detection, battery
//! monitoring and auto-apply of the active profile.

use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use super::state::{AppState, DeviceStatus, Tracked};
use super::tray;
use crate::i18n::{device_name, tr, Text};

const SCAN_INTERVAL: Duration = Duration::from_secs(2);
/// Battery polling while the wireless mouse answers
const BATTERY_INTERVAL_ONLINE: Duration = Duration::from_secs(60);
/// Polling while it sleeps, to notice it waking up quickly
const BATTERY_INTERVAL_OFFLINE: Duration = Duration::from_secs(4);

pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("rivalgui-watcher".into())
        .spawn(move || loop {
            tick(&app);
            std::thread::sleep(SCAN_INTERVAL);
        })
        .expect("cannot spawn watcher thread");
}

/// Notifies the UI and refreshes the tray.
pub fn publish(app: &AppHandle) {
    let state = app.state::<AppState>();
    let _ = app.emit("devices-changed", state.statuses());
    tray::refresh(app);
}

fn tick(app: &AppHandle) {
    let state = app.state::<AppState>();
    let detected = state.hid.lock().unwrap().scan();
    let mut changed = false;
    let mut added = Vec::new();

    {
        let mut devices = state.devices.lock().unwrap();
        let before = devices.len();
        devices.retain(|t| detected.iter().any(|d| d.key == t.status.key));
        changed |= devices.len() != before;

        for d in detected {
            if devices.iter().any(|t| t.status.key == d.key) {
                continue;
            }
            let wireless = d.profile.battery_level.is_some();
            devices.push(Tracked {
                status: DeviceStatus {
                    key: d.key.clone(),
                    name: d.profile.name.replace('\u{a0}', " "),
                    vendor_id: d.profile.vendor_id,
                    product_id: d.profile.product_id,
                    wireless,
                    online: !wireless,
                    battery: None,
                    firmware: None,
                    can_save: d.profile.save_command.is_some(),
                    active_profile: String::new(),
                    error: None,
                },
                detected: d.clone(),
                last_poll: None,
                low_battery_notified: false,
            });
            added.push(d.key.clone());
            changed = true;
        }
    }

    for key in &added {
        state.sync_active_profile(key);
        on_connected(app, key);
    }

    changed |= poll_batteries(app);

    if changed {
        publish(app);
    }
}

fn on_connected(app: &AppHandle, key: &str) {
    let state = app.state::<AppState>();
    let profile = match state.profile_of(key) {
        Ok(p) => p,
        Err(_) => return,
    };
    if profile.firmware_version.is_some() {
        if let Ok(fw) = state.with_device(key, |d| d.firmware_version()) {
            state.update_status(key, |s| s.firmware = fw);
        }
    }
    // Wireless devices are applied once they answer (see poll_batteries)
    if profile.battery_level.is_none() {
        auto_apply(app, key);
    }
}

fn auto_apply(app: &AppHandle, key: &str) {
    let state = app.state::<AppState>();
    if !state.store.lock().unwrap().config.prefs.auto_apply {
        return;
    }
    match state.apply_profile(key, false) {
        Ok(errors) if !errors.is_empty() => eprintln!("rivalgui: auto-apply {key}: {}", errors.join("; ")),
        Err(e) => eprintln!("rivalgui: auto-apply {key}: {e}"),
        _ => {}
    }
}

/// Polls the battery of wireless devices that are due. Returns true if anything changed.
fn poll_batteries(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    let now = Instant::now();
    let lang = state.lang();
    let due: Vec<(String, bool)> = state
        .devices
        .lock()
        .unwrap()
        .iter()
        .filter(|t| t.status.wireless)
        .filter(|t| {
            let interval = if t.status.online { BATTERY_INTERVAL_ONLINE } else { BATTERY_INTERVAL_OFFLINE };
            t.last_poll.is_none_or(|last| now.duration_since(last) >= interval)
        })
        .map(|t| (t.status.key.clone(), t.status.online))
        .collect();

    let mut changed = false;
    for (key, was_online) in due {
        let battery = state.with_device(&key, |d| d.battery()).unwrap_or_default();
        let online = battery.is_known();
        let prefs = state.store.lock().unwrap().config.prefs.clone();
        let mut notify = None;
        {
            let mut devices = state.devices.lock().unwrap();
            let Some(t) = devices.iter_mut().find(|t| t.status.key == key) else { continue };
            t.last_poll = Some(now);
            let new_battery = online.then_some(battery);
            if t.status.battery != new_battery || t.status.online != online {
                changed = true;
            }
            t.status.battery = new_battery;
            t.status.online = online;

            if let (Some(level), Some(charging)) = (battery.level, battery.charging) {
                let low = level <= prefs.low_battery_threshold && !charging;
                if low && !t.low_battery_notified && prefs.notify_low_battery {
                    notify = Some((t.status.name.clone(), level));
                    t.low_battery_notified = true;
                } else if charging || level > prefs.low_battery_threshold.saturating_add(5) {
                    t.low_battery_notified = false;
                }
            }
        }
        if let Some((name, level)) = notify {
            let _ = app
                .notification()
                .builder()
                .title(tr(lang, Text::LowBatteryTitle))
                .body(
                    tr(lang, Text::LowBatteryBody)
                        .replace("{name}", &device_name(lang, &name))
                        .replace("{level}", &level.to_string()),
                )
                .show();
        }
        // Woke up (or first seen awake): restore its configuration
        if online && !was_online {
            auto_apply(app, &key);
        }
    }
    changed
}
