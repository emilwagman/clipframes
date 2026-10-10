//! Letting Claude Code read captures without asking. Claude Code asks before it reads a file
//! outside the project it is working in, and every capture is outside it. One allow rule for
//! the captures folder in Claude Code's own settings file ends the question. Clipframes adds
//! that rule when the user asks for it, and takes away only that rule again.
//!
//! The file belongs to another program, so it is changed as little as possible: one string in
//! `permissions.allow`, everything else as it was, and not at all when it cannot be understood.

use crate::store::Stamp;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// What Settings shows for the switch.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// The rule is in the file.
    On,
    Off,
    /// No folder for Claude Code's settings: it is not set up on this computer.
    Missing,
    /// The captures folder is somewhere a rule cannot name (a network share).
    Unnamed,
    /// The file is there but Clipframes will not change it (not valid JSON, an unexpected
    /// shape, read-only). The user gets the line to add by hand.
    Manual,
}

const FILE: &str = "settings.json";
const BACKUP: &str = "settings.json.clipframes-backup-";
const PART: &str = "settings.json.clipframes-part";

/// One change at a time from this app.
static CHANGING: Mutex<()> = Mutex::new(());

/// Claude Code's settings for this user: in ~/.claude (on Windows %USERPROFILE%\.claude),
/// or in the folder CLAUDE_CONFIG_DIR names when it is set.
pub fn settings_file(home: &Path) -> PathBuf {
    let dir = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|d| !d.is_empty()).map(PathBuf::from);
    dir.unwrap_or_else(|| home.join(".claude")).join(FILE)
}

/// A path the way Claude Code matches it: forward slashes, and a Windows drive as a folder
/// (`C:\Users\sam` is `/c/Users/sam`). None for a network share, which has no such form.
fn posix(path: &str) -> Option<String> {
    let b = path.as_bytes();
    let out = if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        format!("/{}{}", (b[0] as char).to_ascii_lowercase(), path[2..].replace('\\', "/"))
    } else if path.starts_with('/') && !path.starts_with("//") {
        path.to_string()
    } else {
        return None;
    };
    Some(out.trim_end_matches('/').to_string())
}

/// A rule is a gitignore pattern, so the characters that mean something there are escaped.
fn literal(path: &str) -> String {
    let mut out = String::new();
    for c in path.chars() {
        if matches!(c, '\\' | '*' | '?' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The rule that lets Claude Code read everything in `folder`: `Read(~/Clipframes/**)` when
/// the folder is in the home folder, `Read(//Volumes/Work/Captures/**)` (two slashes: from the
/// root of the disk) when it is not. None when the folder cannot be named in a rule.
pub fn rule(folder: &str, home: &str) -> Option<String> {
    // Windows gives some paths with a prefix that only says "exactly as written".
    let (folder, home) = (folder.strip_prefix(r"\\?\").unwrap_or(folder), home.strip_prefix(r"\\?\").unwrap_or(home));
    // Only the separators differ between systems here; a drive letter is handled below.
    let windows = folder.contains('\\') && !folder.starts_with('/');
    let slashed = |p: &str| if windows { p.replace('\\', "/") } else { p.to_string() };
    let (folder, home) = (slashed(folder), slashed(home));
    let (folder, home) = (folder.trim_end_matches('/'), home.trim_end_matches('/'));
    if let Some(inside) = folder.strip_prefix(home).and_then(|rest| rest.strip_prefix('/')) {
        if !home.is_empty() && !inside.is_empty() {
            return Some(format!("Read(~/{}/**)", literal(inside)));
        }
    }
    let whole = posix(folder)?;
    (whole.len() > 1).then(|| format!("Read(/{}/**)", literal(&whole)))
}

/// The rules for a captures folder. One, or two when the folder is a link to another place:
/// Claude Code allows a read through a link only when both the path as written and the place
/// it leads to are allowed.
pub fn rules(folder: &Path, home: &Path) -> Vec<String> {
    let home = home.to_string_lossy();
    let mut rules: Vec<String> = rule(&folder.to_string_lossy(), &home).into_iter().collect();
    if let Some(real) = fs::canonicalize(folder).ok().and_then(|real| rule(&real.to_string_lossy(), &home)) {
        if !rules.is_empty() && !rules.contains(&real) {
            rules.push(real);
        }
    }
    rules
}

/// The file's bytes and what they say, or why it is left alone. Claude Code makes the file
/// only when a setting is first changed, so a folder without one is an empty file not yet
/// written: no bytes, and nothing in it.
fn read(file: &Path) -> Result<(Option<Vec<u8>>, Value), State> {
    let bytes = match fs::read(file) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // A link that leads nowhere is someone's setup, not an absent file.
            let absent = fs::symlink_metadata(file).is_err();
            let folder = file.parent().is_some_and(Path::is_dir);
            return if !folder { Err(State::Missing) } else if absent { Ok((None, json!({}))) } else { Err(State::Manual) };
        }
        Err(_) => return Err(State::Manual),
    };
    // Claude Code's settings are strict JSON. A file with comments, or one that is not text,
    // fails here and is never written. So does one that begins with a byte order mark, and
    // that is meant: unlike Clipframes' own files, this one is another program's. Writing it
    // back would either drop the mark or keep it, and which of the two Claude Code expects
    // is not ours to decide. The switch shows the rule for adding by hand instead.
    let doc: Value = serde_json::from_slice(&bytes).map_err(|_| State::Manual)?;
    let fits = match doc.get("permissions") {
        None => doc.is_object(),
        Some(permissions) => permissions.is_object() && permissions.get("allow").is_none_or(Value::is_array),
    };
    if fits { Ok((Some(bytes), doc)) } else { Err(State::Manual) }
}

fn has(doc: &Value, rule: &str) -> bool {
    doc.pointer("/permissions/allow").and_then(Value::as_array).is_some_and(|list| list.iter().any(|r| r.as_str() == Some(rule)))
}

/// Whether the rules are in the file right now. This is what the switch shows: the file, not
/// something Clipframes remembers.
pub fn state(file: &Path, rules: &[String]) -> State {
    if rules.is_empty() {
        return State::Unnamed;
    }
    match read(file) {
        Ok((_, doc)) => if rules.iter().all(|rule| has(&doc, rule)) { State::On } else { State::Off },
        Err(state) => state,
    }
}

/// Adds the rules or takes them away, and says what the file holds afterwards.
pub fn set(file: &Path, rules: &[String], on: bool) -> State {
    if rules.is_empty() {
        return State::Unnamed;
    }
    let done = if on { change(file, &[], rules) } else { change(file, rules, &[]) };
    done.map_or_else(|state| state, |()| state(file, rules))
}

/// The captures folder changed: the rules for the old one become the rules for the new one.
/// A file without the old rules is left as it is, so the switch stays off.
pub fn moved(file: &Path, old: &[String], new: &[String]) -> State {
    if state(file, old) == State::On && old != new {
        if let Err(state) = change(file, old, new) {
            return state;
        }
    }
    state(file, new)
}

fn change(file: &Path, drop: &[String], add: &[String]) -> Result<(), State> {
    let _one = CHANGING.lock().unwrap_or_else(|e| e.into_inner());
    // A few tries: Claude Code writes this file too, and may do so between our read and write.
    for _ in 0..3 {
        // Rules out a missing folder before anything is resolved or written.
        let there = read(file)?.0.is_some();
        // A settings file that is a link to one kept elsewhere is changed where it is, so the
        // link stays a link. One that does not exist yet is made in the folder as it is.
        let real = if there { fs::canonicalize(file).map_err(|_| State::Manual)? } else { file.to_path_buf() };
        let (bytes, mut doc) = read(&real)?;
        let permissions = match &bytes {
            Some(_) => Some(fs::metadata(&real).map_err(|_| State::Manual)?.permissions()),
            None => None,
        };
        if permissions.as_ref().is_some_and(|p| p.readonly()) {
            return Err(State::Manual);
        }
        let before = doc.clone();
        if let Some(list) = doc.pointer_mut("/permissions/allow").and_then(Value::as_array_mut) {
            list.retain(|r| !r.as_str().is_some_and(|r| drop.iter().any(|d| d == r)));
        }
        for add in add.iter().filter(|add| !has(&before, add) || drop.contains(add)) {
            let root = doc.as_object_mut().ok_or(State::Manual)?;
            let permissions = root.entry("permissions").or_insert_with(|| json!({})).as_object_mut().ok_or(State::Manual)?;
            permissions.entry("allow").or_insert_with(|| json!([])).as_array_mut().ok_or(State::Manual)?.push(add.as_str().into());
        }
        if doc == before {
            return Ok(());
        }
        let mut text = serde_json::to_string_pretty(&doc).map_err(|_| State::Manual)?;
        if bytes.as_ref().is_none_or(|bytes| bytes.ends_with(b"\n")) {
            text.push('\n');
        }
        let dir = real.parent().ok_or(State::Manual)?;
        let part = dir.join(PART);
        let written = fs::write(&part, text).and_then(|()| permissions.map_or(Ok(()), |p| fs::set_permissions(&part, p)));
        // Still the file that was read, or still no file? Then the new one takes its place in
        // one step.
        if written.is_ok() && fs::read(&real).ok() == bytes && (bytes.is_some() || fs::symlink_metadata(&real).is_err()) {
            if bytes.is_some() {
                back_up(&real, dir);
            }
            if fs::rename(&part, &real).is_ok() {
                return Ok(());
            }
        }
        let _ = fs::remove_file(&part);
        if written.is_err() {
            break;
        }
    }
    Err(State::Manual)
}

/// A copy of the file as it was before Clipframes first changed it. Made once, kept.
fn back_up(real: &Path, dir: &Path) {
    let made = fs::read_dir(dir).is_ok_and(|entries| entries.flatten().any(|e| e.file_name().to_string_lossy().starts_with(BACKUP)));
    if !made {
        let _ = fs::copy(real, dir.join(format!("{BACKUP}{}", Stamp::now().folder_name())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULE: &str = "Read(~/Clipframes/**)";

    fn ours() -> Vec<String> {
        vec![RULE.to_string()]
    }

    /// A stand-in for ~/.claude, with `text` as its settings file when there is any.
    fn scratch(name: &str, text: Option<&str>) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("clipframes-claude-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        if let Some(text) = text {
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(FILE), text).unwrap();
        }
        dir.join(FILE)
    }

    fn text(file: &Path) -> String {
        fs::read_to_string(file).unwrap()
    }

    fn backups(file: &Path) -> Vec<PathBuf> {
        fs::read_dir(file.parent().unwrap()).unwrap().flatten().map(|e| e.path()).filter(|p| p.file_name().unwrap().to_string_lossy().starts_with(BACKUP)).collect()
    }

    fn done(file: &Path) {
        fs::remove_dir_all(file.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_folder_in_the_home_folder_is_named_from_there() {
        assert_eq!(rule("/Users/sam/Clipframes", "/Users/sam").as_deref(), Some("Read(~/Clipframes/**)"));
        assert_eq!(rule("/Users/sam/Clipframes/", "/Users/sam/").as_deref(), Some("Read(~/Clipframes/**)"));
        assert_eq!(rule("/Users/sam/Work/My Captures", "/Users/sam").as_deref(), Some("Read(~/Work/My Captures/**)"));
    }

    #[test]
    fn a_folder_elsewhere_is_named_from_the_root_with_two_slashes() {
        assert_eq!(rule("/Volumes/Work/Captures", "/Users/sam").as_deref(), Some("Read(//Volumes/Work/Captures/**)"));
        // Not inside the home folder, only beside a folder that begins the same way.
        assert_eq!(rule("/Users/samantha/Clipframes", "/Users/sam").as_deref(), Some("Read(//Users/samantha/Clipframes/**)"));
        assert_eq!(rule("/", "/Users/sam"), None);
        assert_eq!(rule("/Users/sam", "/Users/sam").as_deref(), Some("Read(//Users/sam/**)"));
    }

    #[test]
    fn a_windows_folder_is_written_the_way_claude_code_matches_it() {
        assert_eq!(rule(r"C:\Users\sam\Clipframes", r"C:\Users\sam").as_deref(), Some("Read(~/Clipframes/**)"));
        assert_eq!(rule(r"D:\Work\Captures", r"C:\Users\sam").as_deref(), Some("Read(//d/Work/Captures/**)"));
        assert_eq!(rule(r"\\?\C:\Captures", r"C:\Users\sam").as_deref(), Some("Read(//c/Captures/**)"));
        // A home folder on a network share still has a folder inside it; one elsewhere on a
        // share has no form the documentation gives.
        assert_eq!(rule(r"\\nas\home\sam\Clipframes", r"\\nas\home\sam").as_deref(), Some("Read(~/Clipframes/**)"));
        assert_eq!(rule(r"\\nas\work\Captures", r"C:\Users\sam"), None);
    }

    #[test]
    fn characters_that_mean_something_in_a_pattern_are_escaped() {
        assert_eq!(rule("/Volumes/Work/[2026] Captures*", "/Users/sam").as_deref(), Some(r"Read(//Volumes/Work/\[2026\] Captures\*/**)"));
        assert_eq!(rule("/Users/sam/Shots (old)", "/Users/sam").as_deref(), Some("Read(~/Shots (old)/**)"));
    }

    #[test]
    fn without_its_folder_claude_code_is_not_there_and_nothing_is_made() {
        let file = scratch("missing", None);
        assert_eq!(state(&file, &ours()), State::Missing);
        assert_eq!(set(&file, &ours(), true), State::Missing);
        assert_eq!(set(&file, &ours(), false), State::Missing);
        assert!(!file.parent().unwrap().exists(), "the folder must not be made");
    }

    #[test]
    fn a_folder_without_a_settings_file_gets_one_holding_only_the_rule() {
        let file = scratch("no-file", None);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        assert_eq!(state(&file, &ours()), State::Off);
        // Off when there is nothing makes nothing.
        assert_eq!(set(&file, &ours(), false), State::Off);
        assert!(!file.exists());
        assert_eq!(set(&file, &ours(), true), State::On);
        assert_eq!(text(&file), "{\n  \"permissions\": {\n    \"allow\": [\n      \"Read(~/Clipframes/**)\"\n    ]\n  }\n}\n");
        assert_eq!(fs::read_dir(file.parent().unwrap()).unwrap().count(), 1, "nothing was there to copy, and nothing is left behind");
        // Off takes the rule out and leaves the file.
        assert_eq!(set(&file, &ours(), false), State::Off);
        assert_eq!(text(&file), "{\n  \"permissions\": {\n    \"allow\": []\n  }\n}\n");
        done(&file);
    }

    #[cfg(unix)]
    #[test]
    fn a_settings_link_that_leads_nowhere_is_left_alone() {
        let file = scratch("dangling", None);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(file.with_file_name("gone.json"), &file).unwrap();
        assert_eq!(set(&file, &ours(), true), State::Manual);
        assert!(fs::symlink_metadata(&file).unwrap().file_type().is_symlink());
        assert!(!file.with_file_name("gone.json").exists());
        done(&file);
    }

    #[test]
    fn a_folder_no_rule_can_name_is_said_to_be_that() {
        let file = scratch("unnamed", Some("{}"));
        assert_eq!(state(&file, &[]), State::Unnamed);
        assert_eq!(set(&file, &[], true), State::Unnamed);
        assert_eq!(text(&file), "{}");
        done(&file);
    }

    #[test]
    fn an_empty_file_gets_the_rule_and_nothing_else() {
        let file = scratch("empty", Some("{}"));
        assert_eq!(state(&file, &ours()), State::Off);
        assert_eq!(set(&file, &ours(), true), State::On);
        assert_eq!(text(&file), "{\n  \"permissions\": {\n    \"allow\": [\n      \"Read(~/Clipframes/**)\"\n    ]\n  }\n}");
        done(&file);
    }

    #[test]
    fn everything_already_in_the_file_stays_in_its_order() {
        let mine = r#"{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "model": "opus",
  "permissions": {
    "deny": [
      "Read(./.env)"
    ],
    "allow": [
      "Bash(npm test *)",
      "Read(~/Documents/*.pdf)"
    ],
    "defaultMode": "acceptEdits"
  },
  "hooks": {
    "Stop": []
  },
  "somethingNew": {
    "z": 1,
    "a": "é ☃"
  }
}
"#;
        let file = scratch("others", Some(mine));
        assert_eq!(set(&file, &ours(), true), State::On);
        assert_eq!(text(&file), mine.replace("\"Read(~/Documents/*.pdf)\"\n", "\"Read(~/Documents/*.pdf)\",\n      \"Read(~/Clipframes/**)\"\n"));
        // Off again is the file as it was, to the byte.
        assert_eq!(set(&file, &ours(), false), State::Off);
        assert_eq!(text(&file), mine);
        done(&file);
    }

    #[test]
    fn switching_on_twice_adds_one_rule_and_writes_once() {
        let file = scratch("twice", Some(r#"{"permissions":{"allow":["Read(~/Clipframes/**)"]}}"#));
        assert_eq!(state(&file, &ours()), State::On);
        assert_eq!(set(&file, &ours(), true), State::On);
        // Already there: not rewritten, so not even reformatted, and no backup.
        assert_eq!(text(&file), r#"{"permissions":{"allow":["Read(~/Clipframes/**)"]}}"#);
        assert!(backups(&file).is_empty());
        done(&file);
    }

    #[test]
    fn switching_off_takes_only_our_rule() {
        let file = scratch("off", Some(r#"{"permissions":{"allow":["Read(~/Clipframes/**)","Bash(ls *)","Read(~/Clipframes/*)","Read(//Users/sam/Clipframes/**)"]}}"#));
        assert_eq!(set(&file, &ours(), false), State::Off);
        let doc: Value = serde_json::from_str(&text(&file)).unwrap();
        assert_eq!(doc["permissions"]["allow"], json!(["Bash(ls *)", "Read(~/Clipframes/*)", "Read(//Users/sam/Clipframes/**)"]));
        // Off when it is already off changes nothing.
        let before = text(&file);
        assert_eq!(set(&file, &ours(), false), State::Off);
        assert_eq!(text(&file), before);
        done(&file);
    }

    #[test]
    fn a_file_that_cannot_be_understood_is_not_touched() {
        for odd in ["{ // mine\n  \"model\": \"opus\"\n}", "{\"model\": \"opus\",}", "", "[]", "\"text\"", r#"{"permissions":[]}"#, r#"{"permissions":{"allow":"Read"}}"#] {
            let file = scratch("odd", Some(odd));
            assert_eq!(state(&file, &ours()), State::Manual, "{odd}");
            assert_eq!(set(&file, &ours(), true), State::Manual, "{odd}");
            assert_eq!(set(&file, &ours(), false), State::Manual, "{odd}");
            assert_eq!(text(&file), odd);
            assert_eq!(fs::read_dir(file.parent().unwrap()).unwrap().count(), 1, "no backup and nothing left behind");
            done(&file);
        }
        // Not text at all.
        let file = scratch("bytes", Some(""));
        fs::write(&file, [0xff, 0xfe, b'{', 0, b'}', 0]).unwrap();
        assert_eq!(set(&file, &ours(), true), State::Manual);
        assert_eq!(fs::read(&file).unwrap(), [0xff, 0xfe, b'{', 0, b'}', 0]);
        done(&file);
    }

    #[test]
    fn a_new_captures_folder_moves_the_rule() {
        let file = scratch("moved", Some(r#"{"permissions":{"allow":["Bash(ls *)","Read(~/Clipframes/**)"]}}"#));
        let new = vec![rule("/Volumes/Work/Captures", "/Users/sam").unwrap()];
        assert_eq!(moved(&file, &ours(), &new), State::On);
        let doc: Value = serde_json::from_str(&text(&file)).unwrap();
        assert_eq!(doc["permissions"]["allow"], json!(["Bash(ls *)", "Read(//Volumes/Work/Captures/**)"]));
        assert_eq!(state(&file, &ours()), State::Off);
        done(&file);
        // Off stays off: a folder change never switches it on.
        let file = scratch("moved-off", Some(r#"{"permissions":{"allow":["Bash(ls *)"]}}"#));
        assert_eq!(moved(&file, &ours(), &new), State::Off);
        assert_eq!(text(&file), r#"{"permissions":{"allow":["Bash(ls *)"]}}"#);
        done(&file);
    }

    #[test]
    fn the_file_is_copied_once_before_the_first_change() {
        let first = r#"{"model":"opus"}"#;
        let file = scratch("backup", Some(first));
        assert_eq!(set(&file, &ours(), true), State::On);
        let made = backups(&file);
        assert_eq!(made.len(), 1);
        assert_eq!(text(&made[0]), first);
        assert_eq!(set(&file, &ours(), false), State::Off);
        assert_eq!(set(&file, &ours(), true), State::On);
        assert_eq!(backups(&file), made, "later changes make no more copies");
        assert_eq!(text(&made[0]), first);
        assert!(!file.with_file_name(PART).exists());
        done(&file);
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_settings_file_is_changed_where_it_is_and_keeps_who_may_read_it() {
        use std::os::unix::fs::PermissionsExt;
        let kept = scratch("dotfiles", Some("{}"));
        fs::set_permissions(&kept, fs::Permissions::from_mode(0o600)).unwrap();
        let file = scratch("linked", None);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&kept, &file).unwrap();
        assert_eq!(set(&file, &ours(), true), State::On);
        assert!(fs::symlink_metadata(&file).unwrap().file_type().is_symlink());
        assert!(text(&kept).contains(RULE));
        assert_eq!(fs::metadata(&kept).unwrap().permissions().mode() & 0o777, 0o600);
        done(&file);
        done(&kept);
    }

    #[cfg(unix)]
    #[test]
    fn a_captures_folder_that_is_a_link_gets_a_rule_for_both_places() {
        let home = scratch("home", None).parent().unwrap().to_path_buf();
        let disk = scratch("disk", None).parent().unwrap().to_path_buf();
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(disk.join("Captures")).unwrap();
        // The scratch folder may itself be reached through a link (/tmp on a Mac).
        let (home, disk) = (fs::canonicalize(home).unwrap(), fs::canonicalize(disk).unwrap());
        assert_eq!(rules(&home.join("Clipframes"), &home), ours(), "a folder not made yet has its one rule");
        std::os::unix::fs::symlink(disk.join("Captures"), home.join("Clipframes")).unwrap();
        let both = rules(&home.join("Clipframes"), &home);
        assert_eq!(both, [RULE.to_string(), format!("Read(/{}/Captures/**)", disk.display())]);
        // On only with both in the file, and off takes both.
        let file = scratch("link-settings", Some(r#"{"permissions":{"allow":["Read(~/Clipframes/**)"]}}"#));
        assert_eq!(state(&file, &both), State::Off);
        assert_eq!(set(&file, &both, true), State::On);
        assert_eq!(set(&file, &both, false), State::Off);
        assert_eq!(serde_json::from_str::<Value>(&text(&file)).unwrap()["permissions"]["allow"], json!([]));
        done(&file);
        fs::remove_dir_all(home).unwrap();
        fs::remove_dir_all(disk).unwrap();
    }

    #[test]
    fn a_read_only_file_is_left_alone() {
        let file = scratch("readonly", Some("{}"));
        let mut permissions = fs::metadata(&file).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&file, permissions.clone()).unwrap();
        assert_eq!(state(&file, &ours()), State::Off);
        assert_eq!(set(&file, &ours(), true), State::Manual);
        assert_eq!(text(&file), "{}");
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(&file, permissions).unwrap();
        done(&file);
    }

    #[test]
    fn the_settings_file_is_in_the_home_folder() {
        // CLAUDE_CONFIG_DIR is left as it is here: other tests run at the same time.
        if std::env::var_os("CLAUDE_CONFIG_DIR").is_none() {
            assert_eq!(settings_file(Path::new("/Users/sam")), Path::new("/Users/sam/.claude/settings.json"));
        }
    }
}
