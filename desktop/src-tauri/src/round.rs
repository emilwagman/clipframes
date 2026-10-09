//! A round: everything picked between opening Clipframes and closing it.
//!
//! Every pick rewrites the clipboard with the whole round so far, so one change and five
//! changes are the same flow. The text is written for an agent to act on directly; the notes
//! file adds the detail and the pictures.

use crate::element::ElementInfo;
use serde::{Deserialize, Serialize};

/// One thing the user pointed at, with what they said about it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pick {
    pub element: ElementInfo,
    /// What the user wants done with it. May be empty.
    pub note: String,
    /// The pick's picture inside the capture folder, e.g. "1.png". Empty until it is saved.
    pub image: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Round {
    pub picks: Vec<Pick>,
}

impl Round {
    pub fn add(&mut self, element: ElementInfo) -> usize {
        self.picks.push(Pick { element, ..Default::default() });
        self.picks.len() - 1
    }

    pub fn set_note(&mut self, index: usize, note: &str) {
        if let Some(p) = self.picks.get_mut(index) {
            p.note = note.trim().to_string();
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.picks.len() {
            self.picks.remove(index);
        }
    }

    /// What goes on the clipboard. `notes_path` is the capture's notes.md once it is saved.
    pub fn reference(&self, notes_path: Option<&str>) -> String {
        match self.picks.as_slice() {
            [] => String::new(),
            [one] => {
                // One pick reads as one line, the way a single capture always has.
                let mut s = format!("[{}{}", describe(&one.element), place(&one.element, " in "));
                if !one.note.is_empty() {
                    s.push_str(&format!(": {}", one.note));
                }
                if let Some(p) = notes_path {
                    s.push_str(&format!(". Read {p}"));
                }
                s.push(']');
                s
            }
            many => {
                let shared = shared_place(many);
                let mut s = format!("[Clipframes: {} things{}", many.len(), shared.as_deref().map(|p| format!(" in {p}")).unwrap_or_default());
                if let Some(p) = notes_path {
                    s.push_str(&format!(". Read {p}"));
                }
                s.push_str("]\n");
                for (i, pick) in many.iter().enumerate() {
                    s.push_str(&format!("{}. {}", i + 1, describe(&pick.element)));
                    if shared.is_none() {
                        s.push_str(&place(&pick.element, " in "));
                    }
                    if !pick.note.is_empty() {
                        s.push_str(&format!(": {}", pick.note));
                    }
                    s.push('\n');
                }
                s.trim_end().to_string()
            }
        }
    }
}

/// `Button "New invoice" (#new-invoice .btn.btn-primary)`
pub(crate) fn describe(e: &ElementInfo) -> String {
    let selector = e.selector();
    if selector.is_empty() { e.headline() } else { format!("{} ({selector})", e.headline()) }
}

/// `Google Chrome "Invoices"`, with a leading word when there is anything to say.
fn place(e: &ElementInfo, lead: &str) -> String {
    let p = place_name(e);
    if p.is_empty() { String::new() } else { format!("{lead}{p}") }
}

pub(crate) fn place_name(e: &ElementInfo) -> String {
    match (e.app.is_empty(), e.window.is_empty() || e.window == e.app) {
        (true, true) => String::new(),
        (true, false) => format!("\"{}\"", e.window),
        (false, true) => e.app.clone(),
        (false, false) => format!("{} \"{}\"", e.app, e.window),
    }
}

/// The place all picks share, if they share one: then it is said once, in the first line.
pub(crate) fn shared_place(picks: &[Pick]) -> Option<String> {
    let first = place_name(&picks.first()?.element);
    (!first.is_empty() && picks.iter().all(|p| place_name(&p.element) == first)).then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button() -> ElementInfo {
        ElementInfo {
            app: "Google Chrome".into(),
            window: "Invoices".into(),
            role: "Button".into(),
            name: "New invoice".into(),
            dom_id: "new-invoice".into(),
            dom_classes: "btn btn-primary".into(),
            ..Default::default()
        }
    }

    fn stat() -> ElementInfo {
        ElementInfo { app: "Google Chrome".into(), window: "Invoices".into(), role: "Group".into(), name: "Overdue".into(), dom_id: "overdue-total".into(), dom_classes: "stat".into(), ..Default::default() }
    }

    #[test]
    fn nothing_picked_copies_nothing() {
        assert_eq!(Round::default().reference(None), "");
    }

    #[test]
    fn one_pick_is_one_line() {
        let mut r = Round::default();
        r.add(button());
        assert_eq!(r.reference(None), "[Button \"New invoice\" (#new-invoice .btn.btn-primary) in Google Chrome \"Invoices\"]");
    }

    #[test]
    fn a_note_and_the_notes_file_join_the_line() {
        let mut r = Round::default();
        let i = r.add(button());
        r.set_note(i, "  make this secondary ");
        assert_eq!(
            r.reference(Some("/Users/sam/Clipframes/2026-10-09_11-42-30/notes.md")),
            "[Button \"New invoice\" (#new-invoice .btn.btn-primary) in Google Chrome \"Invoices\": make this secondary. Read /Users/sam/Clipframes/2026-10-09_11-42-30/notes.md]"
        );
    }

    #[test]
    fn several_picks_in_one_place_name_the_place_once() {
        let mut r = Round::default();
        let a = r.add(button());
        r.set_note(a, "make this secondary");
        r.add(stat());
        assert_eq!(
            r.reference(None),
            "[Clipframes: 2 things in Google Chrome \"Invoices\"]\n1. Button \"New invoice\" (#new-invoice .btn.btn-primary): make this secondary\n2. Group \"Overdue\" (#overdue-total .stat)"
        );
    }

    #[test]
    fn picks_from_different_places_each_say_where() {
        let mut r = Round::default();
        r.add(button());
        r.add(ElementInfo { app: "Slack".into(), window: "general".into(), role: "Tab".into(), name: "Home".into(), ..Default::default() });
        let text = r.reference(None);
        assert!(text.starts_with("[Clipframes: 2 things]\n"), "{text}");
        assert!(text.contains("1. Button \"New invoice\" (#new-invoice .btn.btn-primary) in Google Chrome \"Invoices\""));
        assert!(text.contains("2. Tab \"Home\" in Slack \"general\""));
    }

    #[test]
    fn removing_a_pick_renumbers_the_rest() {
        let mut r = Round::default();
        r.add(button());
        r.add(stat());
        r.remove(0);
        assert_eq!(r.reference(None), "[Group \"Overdue\" (#overdue-total .stat) in Google Chrome \"Invoices\"]");
    }
}
