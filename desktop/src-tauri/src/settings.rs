//! What the user has chosen, kept as one small JSON file in the app's config folder.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

pub const DEFAULT_SHORTCUT: &str = "ctrl+shift+Space";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Modifiers and a key code joined by "+", e.g. "ctrl+shift+Space" or "super+alt+KeyK".
    pub shortcut: String,
    /// Start with the computer, with nothing on screen.
    pub launch_at_login: bool,
    /// Send anonymous counts of what is used and reports of errors.
    pub share_usage: bool,
    /// A random number made on first run, sent with those counts. Empty until then.
    pub install_id: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { shortcut: DEFAULT_SHORTCUT.into(), launch_at_login: true, share_usage: true, install_id: String::new() }
    }
}

const FILE: &str = "settings.json";

/// The saved settings, or the defaults when there are none or the file cannot be read.
/// The second value says whether a file was there: false on the very first run.
pub fn load(dir: &Path) -> (Settings, bool) {
    match fs::read(dir.join(FILE)) {
        Ok(bytes) => (serde_json::from_slice(&bytes).unwrap_or_default(), true),
        Err(_) => (Settings::default(), false),
    }
}

pub fn save(dir: &Path, settings: &Settings) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let part = dir.join("settings.part");
    fs::write(&part, serde_json::to_vec_pretty(settings).map_err(io::Error::other)?)?;
    fs::rename(part, dir.join(FILE))
}

/// How a shortcut is written for people: "⌃⇧Space" on a Mac, "Ctrl+Shift+Space" elsewhere.
pub fn label(shortcut: &str, mac: bool) -> String {
    let mut out: Vec<String> = Vec::new();
    for part in shortcut.split('+') {
        let lower = part.to_ascii_lowercase();
        let word = match lower.as_str() {
            "ctrl" | "control" => if mac { "⌃" } else { "Ctrl" }.to_string(),
            "alt" | "option" => if mac { "⌥" } else { "Alt" }.to_string(),
            "shift" => if mac { "⇧" } else { "Shift" }.to_string(),
            "super" | "cmd" | "command" | "meta" => if mac { "⌘" } else { "Win" }.to_string(),
            _ => part.strip_prefix("Key").or(part.strip_prefix("Digit")).filter(|k| k.len() == 1).unwrap_or(part).to_string(),
        };
        out.push(word);
    }
    if mac {
        out.concat()
    } else {
        out.join("+")
    }
}

/// A shortcut needs a modifier and exactly one key, or it would fire while typing.
pub fn valid(shortcut: &str) -> bool {
    let is_modifier = |p: &str| matches!(p.to_ascii_lowercase().as_str(), "ctrl" | "control" | "alt" | "option" | "shift" | "super" | "cmd" | "command" | "meta");
    let parts: Vec<&str> = shortcut.split('+').collect();
    let keys = parts.iter().filter(|p| !is_modifier(p)).count();
    let modifiers = parts.len() - keys;
    // Shift alone is still typing: Shift+A is a capital letter.
    let only_shift = modifiers == 1 && parts.iter().any(|p| p.eq_ignore_ascii_case("shift"));
    keys == 1 && modifiers >= 1 && !only_shift && parts.iter().all(|p| !p.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("clipframes-settings-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_first_run_gets_the_defaults_and_knows_it_is_the_first() {
        let (settings, existed) = load(&scratch("first"));
        assert_eq!(settings, Settings::default());
        assert!(!existed);
        assert!(settings.launch_at_login);
    }

    #[test]
    fn what_is_saved_comes_back() {
        let dir = scratch("roundtrip");
        let mine = Settings { shortcut: "super+alt+KeyK".into(), launch_at_login: false, share_usage: false, install_id: "abc".into() };
        save(&dir, &mine).unwrap();
        assert_eq!(load(&dir), (mine, true));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_damaged_or_older_file_falls_back_field_by_field() {
        let dir = scratch("damaged");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(FILE), br#"{"shortcut":"ctrl+alt+KeyP"}"#).unwrap();
        assert_eq!(load(&dir).0, Settings { shortcut: "ctrl+alt+KeyP".into(), ..Default::default() });
        fs::write(dir.join(FILE), b"not json").unwrap();
        assert_eq!(load(&dir).0, Settings::default());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn shortcuts_are_written_the_way_each_system_writes_them() {
        assert_eq!(label("ctrl+shift+Space", true), "⌃⇧Space");
        assert_eq!(label("ctrl+shift+Space", false), "Ctrl+Shift+Space");
        assert_eq!(label("super+alt+KeyK", true), "⌘⌥K");
        assert_eq!(label("super+alt+Digit1", false), "Win+Alt+1");
        assert_eq!(label("ctrl+F5", false), "Ctrl+F5");
    }

    #[test]
    fn a_shortcut_needs_a_modifier_and_one_key() {
        assert!(valid("ctrl+shift+Space"));
        assert!(valid("alt+KeyK"));
        assert!(!valid("KeyK"), "a bare key would fire while typing");
        assert!(!valid("shift+KeyK"), "shift and a letter is a capital letter");
        assert!(!valid("ctrl+shift"), "modifiers alone are not a shortcut");
        assert!(!valid("ctrl+KeyK+KeyJ"));
        assert!(!valid(""));
    }
}
