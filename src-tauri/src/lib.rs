pub mod commands;
pub mod core;
pub mod i18n;
pub mod service;

use tauri::{AppHandle, Manager, RunEvent, WebviewWindowBuilder};
use tauri_plugin_autostart::MacosLauncher;

use service::state::AppState;

const MAIN_WINDOW: &str = "main";
/// Passed by the autostart entry: start in the tray without opening the window
const MINIMIZED_FLAG: &str = "--minimized";

/// Shows the main window, creating it if needed. The window (and its webview)
/// is destroyed when closed so the app stays light while living in the tray.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let Some(config) = app.config().app.windows.iter().find(|w| w.label == MAIN_WINDOW) else {
        return;
    };
    match WebviewWindowBuilder::from_config(app, config).and_then(|b| b.build()) {
        Ok(window) => {
            let _ = window.set_focus();
        }
        Err(e) => eprintln!("rivalgui: cannot open window: {e}"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_FLAG]),
        ))
        .plugin(tauri_plugin_notification::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::list_devices,
            commands::get_device,
            commands::set_setting,
            commands::reset_setting,
            commands::run_action,
            commands::save_to_device,
            commands::apply_profile,
            commands::reset_profile,
            commands::activate_profile,
            commands::create_profile,
            commands::rename_profile,
            commands::delete_profile,
            commands::set_prefs,
            commands::set_autostart,
            commands::install_udev_rules,
        ])
        .setup(|app| {
            service::tray::create(app.handle())?;
            service::watcher::spawn(app.handle().clone());
            if !std::env::args().any(|a| a == MINIMIZED_FLAG) {
                show_main_window(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building RivalGUI");

    app.run(|_app, event| {
        // Closing the window keeps the app running in the tray; only "Quit" exits
        if let RunEvent::ExitRequested { code: None, api, .. } = event {
            api.prevent_exit();
        }
    });
}
