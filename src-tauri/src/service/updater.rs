//! Update check and one-click install from GitHub Releases, through tauri-plugin-updater.
//!
//! A couple of minutes after start, then once a day while `prefs.check_updates` is on, the app
//! fetches `latest.json` from the latest GitHub release (nothing else is sent). A newer version
//! shows up in the tray menu and in Preferences, with one notification per version. Installing
//! downloads the package for this kind of install (NSIS, MSI, AppImage, deb or rpm), checks its
//! signature against the `pubkey` in tauri.conf.json, installs it and restarts.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use super::state::AppState;
use super::tray;
use crate::i18n::{tr, Text};

const FIRST_CHECK: Duration = Duration::from_secs(2 * 60);
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
/// How often the background thread looks at the clock (wall clock, so sleep doesn't stretch a day)
const TICK: Duration = Duration::from_secs(60 * 60);
/// Points the check at another latest.json (must be https), for testing
const URL_ENV: &str = "RIVALGUI_UPDATE_URL";
const RELEASES: &str = "https://github.com/Advve/rivalgui/releases";
/// Event telling the UI that the offered update changed (payload: version or null)
pub const UPDATE_EVENT: &str = "update-changed";

#[derive(Default)]
pub struct Updates {
    available: Mutex<Option<Update>>,
    last_check: Mutex<Option<SystemTime>>,
    installing: AtomicBool,
}

impl Updates {
    pub fn installing(&self) -> bool {
        self.installing.load(Ordering::SeqCst)
    }
}

/// The newer version offered to the user: None when there is none or it was skipped.
pub fn offered(app: &AppHandle) -> Option<String> {
    let version = app.state::<Updates>().available.lock().unwrap().as_ref().map(|u| u.version.clone())?;
    let skip = app.state::<AppState>().store.lock().unwrap().config.updates.skip_version.clone();
    (version != skip).then_some(version)
}

/// Asks GitHub for the latest release and remembers it if it is newer. Returns its version.
async fn check(app: &AppHandle) -> Result<Option<String>, String> {
    let mut builder = app.updater_builder();
    if let Ok(url) = std::env::var(URL_ENV) {
        let url = tauri::Url::parse(&url).map_err(|e| e.to_string())?;
        builder = builder.endpoints(vec![url]).map_err(|e| e.to_string())?;
    }
    let updater = builder.build().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    let version = update.as_ref().map(|u| u.version.clone());
    let state = app.state::<Updates>();
    *state.available.lock().unwrap() = update;
    *state.last_check.lock().unwrap() = Some(SystemTime::now());
    Ok(version)
}

/// "Check now" (tray or Preferences): always answers, and brings a skipped version back.
pub async fn check_now(app: &AppHandle) -> Result<Option<String>, String> {
    let version = check(app).await?;
    if let Some(v) = &version {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().unwrap();
        if &store.config.updates.skip_version == v {
            store.config.updates.skip_version.clear();
            let _ = store.save();
        }
    }
    changed(app);
    Ok(version)
}

/// Background checks: shortly after start, then daily while enabled.
pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("rivalgui-updates".into())
        .spawn(move || {
            std::thread::sleep(FIRST_CHECK);
            loop {
                let enabled = app.state::<AppState>().store.lock().unwrap().config.prefs.check_updates;
                let due = app
                    .state::<Updates>()
                    .last_check
                    .lock()
                    .unwrap()
                    .is_none_or(|t| t.elapsed().map_or(true, |e| e >= CHECK_INTERVAL));
                if enabled && due {
                    let before = offered(&app);
                    match tauri::async_runtime::block_on(check(&app)) {
                        Ok(_) if offered(&app) != before => announce(&app),
                        Ok(_) => {}
                        Err(e) => eprintln!("rivalgui: update check: {e}"),
                    }
                }
                std::thread::sleep(TICK);
            }
        })
        .expect("cannot spawn update thread");
}

/// Refreshes the tray and tells the UI.
fn changed(app: &AppHandle) {
    tray::refresh(app);
    let _ = app.emit(UPDATE_EVENT, offered(app));
}

/// A background check found a new version: show it, and notify once per version.
fn announce(app: &AppHandle) {
    changed(app);
    let Some(version) = offered(app) else { return };
    let state = app.state::<AppState>();
    let lang = state.lang();
    {
        let mut store = state.store.lock().unwrap();
        if store.config.updates.notified_version == version {
            return;
        }
        store.config.updates.notified_version = version.clone();
        let _ = store.save();
    }
    notify(app, &tr(lang, Text::UpdateNotifyTitle).replace("{version}", &version), tr(lang, Text::UpdateNotifyBody));
}

pub fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// Downloads, verifies and installs the offered update, then restarts. On Windows the installer
/// takes over and restarts the app itself.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<Updates>();
    let update = { state.available.lock().unwrap().clone() };
    let Some(update) = update else { return Err("no update to install".into()) };
    if state.installing.swap(true, Ordering::SeqCst) {
        return Ok(()); // already installing
    }
    tray::refresh(app);
    match update.download_and_install(|_, _| {}, || {}).await {
        Ok(()) => app.restart(),
        Err(e) => {
            state.installing.store(false, Ordering::SeqCst);
            tray::refresh(app);
            Err(e.to_string())
        }
    }
}

/// Hides the offered version until a newer one comes out.
pub fn skip(app: &AppHandle) {
    let Some(version) = offered(app) else { return };
    {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().unwrap();
        store.config.updates.skip_version = version;
        let _ = store.save();
    }
    changed(app);
}

/// Opens the release page of the offered version (or the releases list).
pub fn open_release_page(app: &AppHandle) {
    let url = match offered(app) {
        Some(v) => format!("{RELEASES}/tag/v{v}"),
        None => format!("{RELEASES}/latest"),
    };
    let result = if cfg!(windows) {
        std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", &url]).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(&url).spawn()
    };
    if let Err(e) = result {
        eprintln!("rivalgui: cannot open {url}: {e}");
    }
}

/// Tray "Install and restart": runs off the menu thread and reports failure as a notification
/// (opening the release page, so the update can still be installed by hand).
pub fn install_from_tray(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = tauri::async_runtime::block_on(install(&app)) {
            eprintln!("rivalgui: installing update: {e}");
            let lang = app.state::<AppState>().lang();
            notify(&app, "RivalGUI", tr(lang, Text::UpdateInstallFailed));
            open_release_page(&app);
        }
    });
}

/// Tray "Check now": the answer comes as a notification.
pub fn check_from_tray(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let lang = app.state::<AppState>().lang();
        match tauri::async_runtime::block_on(check_now(&app)) {
            Ok(Some(v)) => notify(
                &app,
                &tr(lang, Text::UpdateNotifyTitle).replace("{version}", &v),
                tr(lang, Text::UpdateNotifyBody),
            ),
            Ok(None) => notify(
                &app,
                "RivalGUI",
                &tr(lang, Text::UpdateLatest).replace("{version}", &app.package_info().version.to_string()),
            ),
            Err(e) => {
                eprintln!("rivalgui: update check: {e}");
                notify(&app, "RivalGUI", tr(lang, Text::UpdateCheckFailed));
            }
        }
    });
}
