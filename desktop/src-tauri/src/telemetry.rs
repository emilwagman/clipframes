//! Anonymous counts of what is used and reports of what went wrong, so problems can be found
//! and fixed. Never what is on the screen, what was picked, or what was written: every event
//! is a name and a few numbers or yes/no answers, listed in desktop/TELEMETRY.md.
//!
//! Events queue here and leave in small batches from one thread that sleeps while there is
//! nothing to send. Nothing is sent when the setting is off, or in a build without a key.

use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

/// Where events go. PostHog's EU servers.
const HOST: &str = "https://eu.i.posthog.com";
/// The project's key, given at build time. It is a public key: it can only add events.
const KEY: &str = match option_env!("CLIPFRAMES_POSTHOG_KEY") {
    Some(key) => key,
    None => "",
};
/// A batch leaves this long after its first event, or sooner when it is full.
const HOLD: Duration = Duration::from_secs(10);
const FULL: usize = 40;

struct Sink {
    queue: Mutex<Sender<Value>>,
    /// A random number made on first run. It says "the same copy of the app", nothing more.
    install: String,
    /// Sent with every event.
    common: Map<String, Value>,
}

static SINK: OnceLock<Sink> = OnceLock::new();
static ON: AtomicBool = AtomicBool::new(false);
/// How the next round is being opened: "shortcut", "tab", "tray", "launch".
static VIA: Mutex<&'static str> = Mutex::new("other");

/// A new install id: 32 hex digits from the system's random hasher seeds.
pub fn new_install_id() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let part = || RandomState::new().build_hasher().finish();
    format!("{:016x}{:016x}", part(), part())
}

fn endpoint() -> Option<(String, String)> {
    // A test can point events at its own server.
    if let Ok(url) = std::env::var("CLIPFRAMES_TELEMETRY_URL") {
        return Some((url, if KEY.is_empty() { "test".into() } else { KEY.into() }));
    }
    (!KEY.is_empty()).then(|| (format!("{HOST}/batch/"), KEY.to_string()))
}

/// True when this build has somewhere to send events.
pub fn available() -> bool {
    endpoint().is_some()
}

/// Sends a batch and waits for it, but never longer than `patience`. The request runs as a task
/// on the app's runtime; this thread only waits, so it is safe from any thread, including one
/// of the runtime's own (a panic there is reported through here too).
fn post(url: &str, key: &str, batch: Vec<Value>, patience: Duration) {
    let body = json!({ "api_key": key, "batch": batch });
    let url = url.to_string();
    let (done, wait) = channel::<()>();
    tauri::async_runtime::spawn(async move {
        let _ = reqwest::Client::new().post(url).timeout(Duration::from_secs(8)).json(&body).send().await;
        let _ = done.send(());
    });
    let _ = wait.recv_timeout(patience);
}

/// Call once at start. `on` is the user's setting.
pub fn start(install: String, version: &str, on: bool) {
    let Some((url, key)) = endpoint() else { return };
    let (queue, events) = channel::<Value>();
    let mut common = Map::new();
    common.insert("app_version".into(), version.into());
    common.insert("os".into(), std::env::consts::OS.into());
    common.insert("arch".into(), std::env::consts::ARCH.into());
    common.insert("$lib".into(), "clipframes".into());
    // No person profiles and no location lookup: these are counts, not people.
    common.insert("$process_person_profile".into(), false.into());
    common.insert("$geoip_disable".into(), true.into());
    if SINK.set(Sink { queue: Mutex::new(queue), install, common }).is_err() {
        return;
    }
    ON.store(on, Ordering::SeqCst);
    thread::spawn(move || {
        // Blocks until there is something to send, so an idle app does nothing here.
        while let Ok(first) = events.recv() {
            let mut batch = vec![first];
            let until = std::time::Instant::now() + HOLD;
            while batch.len() < FULL {
                match events.recv_timeout(until.saturating_duration_since(std::time::Instant::now())) {
                    Ok(next) => batch.push(next),
                    Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => break,
                }
            }
            post(&url, &key, batch, Duration::from_secs(10));
        }
    });
    // A crash is reported before the process goes.
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // The usual report first: whatever happens below, the reason is on stderr.
        before(info);
        let message = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
        let at = info.location().map(|l| format!("{}:{}", l.file().rsplit(['/', '\\']).next().unwrap_or(""), l.line())).unwrap_or_default();
        if let (Some(event), Some((url, key))) = (build("$exception", exception("panic", &message, &at)), endpoint()) {
            post(&url, &key, vec![event], Duration::from_secs(3));
        }
    }));
}

/// The user's setting changed.
pub fn set_enabled(on: bool) {
    ON.store(on, Ordering::SeqCst);
}

fn build(name: &str, properties: Value) -> Option<Value> {
    let sink = SINK.get()?;
    if !ON.load(Ordering::SeqCst) {
        return None;
    }
    let mut all = sink.common.clone();
    if let Value::Object(own) = properties {
        all.extend(own);
    }
    Some(json!({ "event": name, "distinct_id": sink.install, "properties": all }))
}

/// Records that something happened. `properties` is a JSON object of numbers and flags.
pub fn event(name: &str, properties: Value) {
    if let (Some(event), Some(sink)) = (build(name, properties), SINK.get()) {
        let _ = sink.queue.lock().unwrap().send(event);
    }
}

fn exception(kind: &str, message: &str, at: &str) -> Value {
    let message: String = message.chars().take(300).collect();
    json!({
        "$exception_list": [{ "type": kind, "value": message, "mechanism": { "handled": kind != "panic", "synthetic": false } }],
        "$exception_fingerprint": format!("{kind}:{at}:{}", message.chars().take(60).collect::<String>()),
        "at": at,
    })
}

/// Records an error: something that should have worked and did not.
pub fn error(kind: &str, message: &str, at: &str) {
    event("$exception", exception(kind, message, at));
}

/// Says how the round about to open was asked for.
pub fn via(how: &'static str) {
    *VIA.lock().unwrap() = how;
}

/// Reads and clears what `via` said.
pub fn take_via() -> &'static str {
    std::mem::replace(&mut *VIA.lock().unwrap(), "other")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_ids_are_long_and_differ() {
        let (a, b) = (new_install_id(), new_install_id());
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
    }

    #[test]
    fn an_error_report_is_short_and_grouped_by_where_it_happened() {
        let report = exception("capture", &"x".repeat(1000), "shot.rs:10");
        assert_eq!(report["$exception_list"][0]["value"].as_str().unwrap().len(), 300);
        assert!(report["$exception_fingerprint"].as_str().unwrap().starts_with("capture:shot.rs:10:"));
    }
}
