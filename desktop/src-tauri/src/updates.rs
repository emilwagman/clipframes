//! Updates install themselves. A check runs shortly after start and every few hours; a new
//! version is downloaded in the background, verified against the key in tauri.conf.json, and
//! installed at a moment when nothing of Clipframes is on screen. Then the app restarts hidden.

use crate::app;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

const FIRST_CHECK: Duration = Duration::from_secs(30);
const BETWEEN_CHECKS: Duration = Duration::from_secs(6 * 60 * 60);
/// Left in the config folder across the restart, so the new version starts without the bar.
const MARKER: &str = "updated";

static BUSY: AtomicBool = AtomicBool::new(false);

/// Checks in the background for as long as the app runs.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(FIRST_CHECK);
        loop {
            if let Err(e) = check(&app) {
                eprintln!("update check: {e}");
            }
            thread::sleep(BETWEEN_CHECKS);
        }
    });
}

/// True once, right after an update restarted the app.
pub fn just_updated(app: &AppHandle) -> bool {
    let Ok(dir) = app.path().app_config_dir() else { return false };
    std::fs::remove_file(dir.join(MARKER)).is_ok()
}

/// One check, and the install if there is something newer. Returns what to tell the user.
/// When an update is installed this does not return: the app restarts.
pub fn check(app: &AppHandle) -> Result<String, String> {
    if BUSY.swap(true, Ordering::SeqCst) {
        return Ok("Already checking.".into());
    }
    let result = run(app);
    BUSY.store(false, Ordering::SeqCst);
    if let Err(e) = &result {
        crate::telemetry::error("update", e, "updates::check");
    }
    app::set_update_status(app, match &result {
        Ok(message) => message.clone(),
        Err(e) => format!("Could not check for updates: {e}"),
    });
    result
}

fn run(app: &AppHandle) -> Result<String, String> {
    let text = |e: tauri_plugin_updater::Error| e.to_string();
    let updater = app.updater().map_err(text)?;
    let Some(update) = tauri::async_runtime::block_on(updater.check()).map_err(text)? else {
        return Ok(format!("Clipframes {} is the latest version.", app.package_info().version));
    };
    crate::telemetry::event("update_found", serde_json::json!({ "to": update.version }));
    app::set_update_status(app, format!("Downloading version {}…", update.version));
    let bytes = tauri::async_runtime::block_on(update.download(|_, _| {}, || {})).map_err(text)?;

    app::set_update_status(app, format!("Version {} installs as soon as Clipframes is closed.", update.version));
    while !app::idle(app) {
        thread::sleep(Duration::from_secs(3));
    }
    if let Ok(dir) = app.path().app_config_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(MARKER), update.version.as_bytes());
    }
    crate::telemetry::flush();
    // On Windows the installer takes over from here and starts the new version itself.
    if let Err(e) = update.install(bytes) {
        // Nothing was installed, so the next start is an ordinary one.
        if let Ok(dir) = app.path().app_config_dir() {
            let _ = std::fs::remove_file(dir.join(MARKER));
        }
        return Err(text(e));
    }
    app.restart();
}
