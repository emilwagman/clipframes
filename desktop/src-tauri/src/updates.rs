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
        if worth_reporting(&e.kind) {
            // Only which kind of error: its text can hold a path on this computer.
            crate::telemetry::error("update", &e.kind, "updates::check");
        }
    }
    app::set_update_status(app, match &result {
        Ok(message) => message.clone(),
        Err(e) => format!("Could not check for updates: {}", e.text),
    });
    result.map_err(|e| e.text)
}

/// Why an update did not happen: `text` for the user, `kind` for the error report.
struct Failure {
    kind: String,
    text: String,
}

/// The name of an error's variant from its debug form: "Reqwest" from
/// `Reqwest(reqwest::Error { … })`.
fn kind_of(debug: &str) -> String {
    debug.chars().take_while(char::is_ascii_alphanumeric).collect()
}

/// Being offline ("Reqwest") and there being no release to find yet ("ReleaseNotFound") are
/// not things that went wrong in Clipframes.
fn worth_reporting(kind: &str) -> bool {
    !matches!(kind, "Reqwest" | "ReleaseNotFound")
}

fn run(app: &AppHandle) -> Result<String, Failure> {
    let text = |e: tauri_plugin_updater::Error| Failure { kind: kind_of(&format!("{e:?}")), text: e.to_string() };
    let updater = app.updater().map_err(text)?;
    let Some(update) = tauri::async_runtime::block_on(updater.check()).map_err(text)? else {
        return Ok(format!("Clipframes {} is the latest version.", app.package_info().version));
    };
    crate::telemetry::event("update_found", serde_json::json!({ "to": update.version }));
    app::set_update_status(app, format!("Downloading version {}…", update.version));
    let bytes = tauri::async_runtime::block_on(update.download(|_, _| {}, || {})).map_err(text)?;

    app::set_update_status(app, format!("Version {} installs as soon as Clipframes is closed.", update.version));
    // Reports waiting to be sent leave first, which takes a moment, and only then is it
    // decided that nothing is on screen: a round opened in that moment must not be restarted
    // from under the user.
    loop {
        while !app::idle(app) {
            thread::sleep(Duration::from_secs(3));
        }
        crate::telemetry::flush();
        if app::idle(app) {
            break;
        }
    }
    if let Ok(dir) = app.path().app_config_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(MARKER), update.version.as_bytes());
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_update_error_is_reported_by_its_kind_not_its_text() {
        let lost = tauri_plugin_updater::Error::PreviousAppNotRestored("/var/folders/ab/sam/T/clipframes-old".into());
        assert_eq!(kind_of(&format!("{lost:?}")), "PreviousAppNotRestored");
        assert_eq!(kind_of(&format!("{:?}", tauri_plugin_updater::Error::ReleaseNotFound)), "ReleaseNotFound");
        assert_eq!(kind_of("Reqwest(reqwest::Error { kind: Request, url: \"https://github.com/\" })"), "Reqwest");
    }

    #[test]
    fn being_offline_or_having_no_release_yet_is_not_reported() {
        assert!(!worth_reporting("Reqwest"));
        assert!(!worth_reporting("ReleaseNotFound"));
        assert!(worth_reporting("Minisign"));
        assert!(worth_reporting("PreviousAppNotRestored"));
    }
}
