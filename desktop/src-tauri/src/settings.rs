//! What the user has chosen, kept as one small JSON file in the app's config folder.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    /// Show the tab where Clipframes was used before. Off: no tab anywhere, and nothing looks
    /// at which app is in front.
    pub show_tab: bool,
    /// Where the bar was dragged to. By the displays that were connected, then by the display
    /// it was on: how far along that display's free room it sat, 0 to 1 each way.
    pub bar_places: BTreeMap<String, BTreeMap<String, (f64, f64)>>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { shortcut: DEFAULT_SHORTCUT.into(), launch_at_login: true, share_usage: true, install_id: String::new(), show_tab: true, bar_places: BTreeMap::new() }
    }
}

const FILE: &str = "settings.json";

/// A file's bytes without the byte order mark that Notepad, PowerShell and other Windows
/// tools put in front of UTF-8 text. A JSON reader takes the mark for a mistake in the file.
pub fn without_mark(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes)
}

/// Moves a file that is there but cannot be understood out of the way, so that what the
/// user had is kept when a new one is written in its place: "settings.json" becomes
/// "settings.unreadable.json", or "settings.unreadable-2.json" when that name is taken.
pub fn set_aside(dir: &Path, file: &str) {
    let stem = file.strip_suffix(".json").unwrap_or(file);
    let free = (1..100).map(|n| dir.join(if n == 1 { format!("{stem}.unreadable.json") } else { format!("{stem}.unreadable-{n}.json") })).find(|path| !path.exists());
    if let Some(aside) = free {
        if let Err(e) = fs::rename(dir.join(file), &aside) {
            eprintln!("could not keep the unreadable {file} as {}: {e}", aside.display());
        }
    }
}

/// The install number in a settings file that cannot be read as a whole: one stray
/// character elsewhere in the file must not make this copy of the app count as a new one.
/// Looks for the `"installId": "…"` pair in the text, also when every character takes two
/// bytes (PowerShell's own way of writing a file).
fn install_id_in(bytes: &[u8]) -> Option<String> {
    let text: String = String::from_utf8_lossy(bytes).chars().filter(|c| *c != '\0').collect();
    let after = text.split_once("\"installId\"")?.1.trim_start().strip_prefix(':')?.trim_start().strip_prefix('"')?;
    let id = after.split_once('"')?.0;
    (!id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).then(|| id.to_string())
}

/// The saved settings, or the defaults when there are none or the file cannot be read.
/// The second value says whether settings were read from a file: false on the very first
/// run, and for a file that could not be understood. That one is kept beside the new one
/// under another name (`set_aside`), never written over, and gives its install number if
/// it still can.
pub fn load(dir: &Path) -> (Settings, bool) {
    let Ok(bytes) = fs::read(dir.join(FILE)) else { return (Settings::default(), false) };
    match serde_json::from_slice(without_mark(&bytes)) {
        Ok(settings) => (settings, true),
        Err(e) => {
            eprintln!("settings.json could not be read ({e}): kept aside, starting from the defaults");
            set_aside(dir, FILE);
            (Settings { install_id: install_id_in(&bytes).unwrap_or_default(), ..Default::default() }, false)
        }
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
        let on_the_left = BTreeMap::from([("Monitor #1".to_string(), (0.0, 0.25))]);
        let mine = Settings { shortcut: "super+alt+KeyK".into(), launch_at_login: false, share_usage: false, install_id: "abc".into(), show_tab: false, bar_places: BTreeMap::from([("Monitor #1 + Monitor #2".to_string(), on_the_left)]) };
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
        assert!(load(&dir).0.show_tab, "a file from before the switch existed has the tab on");
        fs::write(dir.join(FILE), b"not json").unwrap();
        assert_eq!(load(&dir).0, Settings::default());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_saved_by_a_windows_tool_with_a_byte_order_mark_is_read() {
        let dir = scratch("mark");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(FILE), b"\xEF\xBB\xBF{\r\n  \"shortcut\": \"ctrl+alt+KeyP\",\r\n  \"installId\": \"abc-123\"\r\n}").unwrap();
        assert_eq!(load(&dir), (Settings { shortcut: "ctrl+alt+KeyP".into(), install_id: "abc-123".into(), ..Default::default() }, true));
        assert!(!dir.join("settings.unreadable.json").exists(), "nothing was wrong with it");
        assert_eq!(without_mark(b"{}"), b"{}");
        assert_eq!(without_mark(b""), b"");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_that_cannot_be_read_is_kept_aside_and_not_written_over() {
        let dir = scratch("aside");
        fs::create_dir_all(&dir).unwrap();
        // A comma too many, after an edit by hand.
        let broken = br#"{ "shortcut": "ctrl+alt+KeyP", "installId": "0f3a9c2e-77", }"#;
        fs::write(dir.join(FILE), broken).unwrap();
        let (settings, read) = load(&dir);
        assert!(!read, "so the app writes a new file");
        assert_eq!(settings, Settings { install_id: "0f3a9c2e-77".into(), ..Default::default() }, "the defaults, for the same copy of the app");
        assert!(!dir.join(FILE).exists());
        assert_eq!(fs::read(dir.join("settings.unreadable.json")).unwrap(), broken);
        // What the app does next, and a second file that breaks later: the first one stays.
        save(&dir, &settings).unwrap();
        assert_eq!(load(&dir), (settings, true));
        fs::write(dir.join(FILE), b"again not json").unwrap();
        load(&dir);
        assert_eq!(fs::read(dir.join("settings.unreadable.json")).unwrap(), broken);
        assert_eq!(fs::read(dir.join("settings.unreadable-2.json")).unwrap(), b"again not json");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_install_number_is_found_in_a_file_that_is_otherwise_unreadable() {
        assert_eq!(install_id_in(br#"{"installId":"abc-123","shortcut": oops"#).as_deref(), Some("abc-123"));
        assert_eq!(install_id_in(b"{\n  \"installId\" :\n \"abc-123\"").as_deref(), Some("abc-123"));
        // Written with two bytes a character, mark first: PowerShell's `>`.
        let wide: Vec<u8> = [0xFF, 0xFE].into_iter().chain(r#"{"installId": "abc-123"}"#.encode_utf16().flat_map(u16::to_le_bytes)).collect();
        assert_eq!(install_id_in(&wide).as_deref(), Some("abc-123"));
        assert_eq!(install_id_in(b"not json"), None);
        assert_eq!(install_id_in(br#"{"installId": 5}"#), None);
        assert_eq!(install_id_in(br#"{"installId": "two words"}"#), None, "not something this app wrote");
        assert_eq!(install_id_in(br#"{"installId": ""}"#), None);
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
