//! System tray icon: battery indicator, device status, profile switching.

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_autostart::ManagerExt;

use super::state::{AppState, DeviceStatus};
use crate::i18n::{device_name, profile_name, tr, Lang, Text};

const TRAY_ID: &str = "main";
const BASE_ICON: &[u8] = include_bytes!("../../icons/32x32.png");

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(BASE_ICON)?)
        .tooltip("RivalGUI")
        .menu(&build_menu(app, &[])?)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Rebuilds menu, tooltip and icon from the current state.
pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let devices = app.state::<AppState>().statuses();
    let lang = app.state::<AppState>().lang();
    if let Ok(menu) = build_menu(app, &devices) {
        let _ = tray.set_menu(Some(menu));
    }
    let _ = tray.set_tooltip(Some(tooltip(lang, &devices)));
    let battery = devices.iter().find_map(|d| d.battery);
    if let Ok(icon) = battery_icon(battery.and_then(|b| b.level), battery.and_then(|b| b.charging)) {
        let _ = tray.set_icon(Some(icon));
    }
}

pub fn status_line(lang: Lang, d: &DeviceStatus) -> String {
    let state = match (d.wireless, d.online, d.battery) {
        (_, _, Some(b)) if b.level.is_some() => {
            let charging = if b.charging == Some(true) { " ⚡" } else { "" };
            format!("{}%{charging}", b.level.unwrap_or(0))
        }
        (true, false, _) => tr(lang, Text::Asleep).into(),
        _ => tr(lang, Text::Connected).into(),
    };
    format!("{} - {state}", device_name(lang, short_name(&d.name)))
}

fn short_name(name: &str) -> &str {
    name.strip_prefix("SteelSeries ").unwrap_or(name)
}

fn tooltip(lang: Lang, devices: &[DeviceStatus]) -> String {
    if devices.is_empty() {
        return tr(lang, Text::TooltipNoMouse).into();
    }
    let lines: Vec<String> = devices.iter().map(|d| status_line(lang, d)).collect();
    format!("RivalGUI\n{}", lines.join("\n"))
}

fn build_menu(app: &AppHandle, devices: &[DeviceStatus]) -> tauri::Result<Menu<Wry>> {
    let state = app.state::<AppState>();
    let lang = state.lang();
    let mut items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();

    if devices.is_empty() {
        items.push(Box::new(MenuItem::with_id(app, "none", tr(lang, Text::NoMouse), false, None::<&str>)?));
    }
    for d in devices {
        items.push(Box::new(MenuItem::with_id(app, format!("dev|{}", d.key), status_line(lang, d), false, None::<&str>)?));
    }
    for d in devices {
        let (names, active) = {
            let mut store = state.store.lock().unwrap();
            let config = store.device(&d.key);
            (config.profile_names(), config.active.clone())
        };
        let mut entries: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();
        for name in names {
            entries.push(Box::new(CheckMenuItem::with_id(
                app,
                format!("profile|{}|{}", d.key, name),
                profile_name(lang, &name),
                true,
                name == active,
                None::<&str>,
            )?));
        }
        entries.push(Box::new(PredefinedMenuItem::separator(app)?));
        entries.push(Box::new(MenuItem::with_id(
            app,
            format!("reapply|{}", d.key),
            tr(lang, Text::Reapply),
            true,
            None::<&str>,
        )?));
        let refs: Vec<&dyn IsMenuItem<Wry>> = entries.iter().map(|b| b.as_ref()).collect();
        let title = if devices.len() == 1 {
            tr(lang, Text::Profile).to_string()
        } else {
            tr(lang, Text::ProfileOf).replace("{name}", &device_name(lang, short_name(&d.name)))
        };
        items.push(Box::new(Submenu::with_items(app, title, true, &refs)?));
    }

    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    items.push(Box::new(MenuItem::with_id(app, "open", tr(lang, Text::Open), true, None::<&str>)?));
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);
    items.push(Box::new(CheckMenuItem::with_id(app, "autostart", tr(lang, Text::StartWithSystem), true, autostart, None::<&str>)?));
    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    items.push(Box::new(MenuItem::with_id(app, "quit", tr(lang, Text::Quit), true, None::<&str>)?));

    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|b| b.as_ref()).collect();
    Menu::with_items(app, &refs)
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref().to_string();
    let parts: Vec<&str> = id.splitn(3, '|').collect();
    match parts.as_slice() {
        ["open"] => crate::show_main_window(app),
        ["quit"] => app.exit(0),
        ["autostart"] => {
            let launcher = app.autolaunch();
            let _ = if launcher.is_enabled().unwrap_or(false) { launcher.disable() } else { launcher.enable() };
            super::watcher::publish(app);
        }
        ["profile", key, name] => {
            let (app, key, name) = (app.clone(), key.to_string(), name.to_string());
            std::thread::spawn(move || {
                let _ = crate::commands::activate_profile_impl(&app, &key, &name);
            });
        }
        ["reapply", key] => {
            let (app, key) = (app.clone(), key.to_string());
            std::thread::spawn(move || {
                let _ = app.state::<AppState>().apply_profile(&key, true);
                super::watcher::publish(&app);
            });
        }
        _ => {}
    }
}

/// The app icon with a battery gauge drawn at the bottom.
fn battery_icon(level: Option<u8>, charging: Option<bool>) -> tauri::Result<Image<'static>> {
    let base = Image::from_bytes(BASE_ICON)?;
    let (w, h) = (base.width() as usize, base.height() as usize);
    let mut rgba = base.rgba().to_vec();
    if let Some(level) = level {
        let color: [u8; 3] = if charging == Some(true) {
            [0x4d, 0xa3, 0xff]
        } else if level > 50 {
            [0x3d, 0xdc, 0x84]
        } else if level > 20 {
            [0xff, 0xb0, 0x20]
        } else {
            [0xff, 0x4d, 0x4f]
        };
        let (x0, x1) = (w / 8, w - w / 8);
        let (y0, y1) = (h - h / 5, h - h / 16);
        let filled = x0 + ((x1 - x0) * level.min(100) as usize).div_ceil(100);
        for y in y0..y1 {
            for x in x0..x1 {
                let i = (y * w + x) * 4;
                let border = y == y0 || y == y1 - 1 || x == x0 || x == x1 - 1;
                let px: [u8; 4] = if border {
                    [0x0d, 0x10, 0x17, 0xff]
                } else if x < filled {
                    [color[0], color[1], color[2], 0xff]
                } else {
                    [0x2a, 0x30, 0x3c, 0xff]
                };
                rgba[i..i + 4].copy_from_slice(&px);
            }
        }
    }
    Ok(Image::new_owned(rgba, w as u32, h as u32))
}
