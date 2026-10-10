//! A round: everything picked between opening Clipframes and closing it.
//!
//! Every pick rewrites the clipboard with the whole round so far, so one change and five
//! changes are the same flow. The text is written for an agent to act on directly; the notes
//! file adds the detail and the pictures.

use crate::element::ElementInfo;
use serde::{Deserialize, Serialize};

/// The ways of showing something: an element pointed at, a picture of an area, a clip of it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Element,
    Area,
    Clip,
}

/// A click made while a clip was being recorded.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Click {
    /// Seconds into the clip.
    pub at: f64,
    /// The first frame taken after the click.
    pub frame: u32,
    /// What was clicked, e.g. `Button "Save"`. Empty when nothing could be read there.
    pub what: String,
}

/// One thing the user showed, with what they said about it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Pick {
    /// What the pick is known by while its round is in the app, so that something learned
    /// about it later finds it wherever picks removed since have moved it. Zero when it has
    /// none; not kept on disk.
    #[serde(skip)]
    pub id: u64,
    pub kind: Kind,
    /// The element picked. For an area or a clip: where it was taken (app, window, page) and
    /// the area itself as the frame.
    pub element: ElementInfo,
    /// What the user wants done with it. May be empty.
    pub note: String,
    /// The picture inside the capture folder, e.g. "1.png". For a clip, the folder of its
    /// frames, e.g. "3". Empty when no picture could be taken.
    pub image: String,
    /// Width and height of the picture, or of each frame, in pixels.
    pub pixels: (u32, u32),
    /// Clips only: how many frames, how long, and the clicks made meanwhile.
    pub frames: u32,
    pub seconds: f64,
    pub clicks: Vec<Click>,
}

impl Pick {
    /// `Button "New invoice"`, `Screenshot`, `Screen clip, 6 s`.
    pub fn headline(&self) -> String {
        match self.kind {
            Kind::Element => self.element.headline(),
            Kind::Area => "Screenshot".into(),
            Kind::Clip => format!("Screen clip, {} s", self.seconds.round().max(1.0) as u32),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Round {
    pub picks: Vec<Pick>,
}

impl Round {
    pub fn add(&mut self, element: ElementInfo) -> usize {
        self.push(Pick { element, ..Default::default() })
    }

    pub fn push(&mut self, pick: Pick) -> usize {
        self.picks.push(pick);
        self.picks.len() - 1
    }

    pub fn set_note(&mut self, index: usize, note: &str) {
        if let Some(p) = self.picks.get_mut(index) {
            p.note = note.trim().to_string();
        }
    }

    /// Adds to the pick known as `id` which one it is among those that read the same and
    /// what heading it is under, learned after the pick was made. False when that pick is no
    /// longer in the round.
    pub fn locate(&mut self, id: u64, occurrence: Option<(u32, u32)>, heading: Option<crate::element::Heading>) -> bool {
        match self.picks.iter_mut().find(|p| id != 0 && p.id == id) {
            Some(pick) => {
                pick.element.occurrence = occurrence;
                pick.element.heading = heading;
                true
            }
            None => false,
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
                let mut s = format!("[{}{}", describe(one), place(&one.element, " in "));
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
                    s.push_str(&format!("{}. {}", i + 1, describe(pick)));
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

/// `Button "New invoice" (#new-invoice .btn.btn-primary)`, `Screenshot (2.png)`,
/// `Screen clip, 6 s, 24 frames (3/)`. An element that reads the same as others on its page
/// also says which one it is and what heading it is under:
/// `Link "Download" (.pill), 2nd of 2 on the page, under heading "Try it on your own app."`.
pub(crate) fn describe(pick: &Pick) -> String {
    match pick.kind {
        Kind::Element => {
            let selector = pick.element.selector();
            let mut said = if selector.is_empty() { pick.element.headline() } else { format!("{} ({selector})", pick.element.headline()) };
            // What it is, then which one, then under what.
            for clause in pick.element.whereabouts() {
                said.push_str(&format!(", {clause}"));
            }
            said
        }
        Kind::Area if pick.image.is_empty() => "Screenshot".into(),
        Kind::Area => format!("Screenshot ({})", pick.image),
        Kind::Clip => format!("{}, {} frames ({}/)", pick.headline(), pick.frames, pick.image),
    }
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

    fn second_download() -> ElementInfo {
        use crate::element::Heading;
        ElementInfo {
            app: "Google Chrome".into(),
            window: "Clipframes".into(),
            url: "https://clipframes.com/".into(),
            role: "Link".into(),
            name: "Download for Windows".into(),
            dom_classes: "home_pill__qnvOg home_big__1QvCE".into(),
            occurrence: Some((2, 2)),
            heading: Some(Heading { text: "Try it on your own app.".into(), inside: false }),
            ..Default::default()
        }
    }

    #[test]
    fn one_of_several_that_read_the_same_says_which_one_and_under_what_heading() {
        let mut r = Round::default();
        let i = r.add(second_download());
        r.set_note(i, "change this to say Download for PC");
        assert_eq!(
            r.reference(Some("/Users/sam/Clipframes/2026-10-10_09-21-47/notes.md")),
            "[Link \"Download for Windows\" (.home_pill__qnvOg.home_big__1QvCE), 2nd of 2 on the page, under heading \"Try it on your own app.\" in Google Chrome \"Clipframes\": change this to say Download for PC. Read /Users/sam/Clipframes/2026-10-10_09-21-47/notes.md]"
        );
    }

    #[test]
    fn in_a_numbered_list_each_line_carries_its_own_which_one_and_heading() {
        use crate::element::Heading;
        let mut r = Round::default();
        r.add(second_download());
        // A native app: no page, and a section that holds its own heading. No selector.
        r.add(ElementInfo { app: "Google Chrome".into(), window: "Clipframes".into(), role: "Group".into(), heading: Some(Heading { text: "Questions".into(), inside: true }), ..Default::default() });
        r.add(ElementInfo { app: "Google Chrome".into(), window: "Clipframes".into(), role: "Button".into(), name: "OK".into(), occurrence: Some((1, 3)), ..Default::default() });
        r.push(Pick { kind: Kind::Area, element: ElementInfo { occurrence: Some((1, 2)), ..chrome_site() }, image: "4.png".into(), ..Default::default() });
        assert_eq!(
            r.reference(None),
            "[Clipframes: 4 things in Google Chrome \"Clipframes\"]\n1. Link \"Download for Windows\" (.home_pill__qnvOg.home_big__1QvCE), 2nd of 2 on the page, under heading \"Try it on your own app.\"\n2. Group, with heading \"Questions\"\n3. Button \"OK\", 1st of 3 in the window\n4. Screenshot (4.png)"
        );
    }

    fn chrome_site() -> ElementInfo {
        ElementInfo { app: "Google Chrome".into(), window: "Clipframes".into(), ..Default::default() }
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

    fn chrome() -> ElementInfo {
        ElementInfo { app: "Google Chrome".into(), window: "Invoices".into(), ..Default::default() }
    }

    #[test]
    fn a_round_can_mix_elements_screenshots_and_clips() {
        let mut r = Round::default();
        let a = r.add(button());
        r.set_note(a, "make this green");
        let b = r.push(Pick { kind: Kind::Area, element: chrome(), image: "2.png".into(), pixels: (800, 400), ..Default::default() });
        r.set_note(b, "the table is cramped");
        r.push(Pick { kind: Kind::Clip, element: chrome(), image: "3".into(), frames: 24, seconds: 6.2, ..Default::default() });
        assert_eq!(
            r.reference(None),
            "[Clipframes: 3 things in Google Chrome \"Invoices\"]\n1. Button \"New invoice\" (#new-invoice .btn.btn-primary): make this green\n2. Screenshot (2.png): the table is cramped\n3. Screen clip, 6 s, 24 frames (3/)"
        );
    }

    #[test]
    fn a_screenshot_alone_is_one_line() {
        let mut r = Round::default();
        r.push(Pick { kind: Kind::Area, element: chrome(), image: "1.png".into(), ..Default::default() });
        assert_eq!(r.reference(None), "[Screenshot (1.png) in Google Chrome \"Invoices\"]");
    }

    #[test]
    fn rounds_saved_before_there_were_kinds_still_load() {
        let old = r#"{"picks":[{"element":{"role":"Button","name":"Save"},"note":"x","image":""}]}"#;
        let r: Round = serde_json::from_str(old).unwrap();
        assert_eq!(r.picks[0].kind, Kind::Element);
        assert_eq!(r.picks[0].headline(), "Button \"Save\"");
    }

    fn paid(id: u64) -> Pick {
        Pick { id, element: ElementInfo { role: "Text".into(), name: "Paid".into(), url: "http://localhost:3000/".into(), ..chrome() }, ..Default::default() }
    }

    #[test]
    fn what_is_learned_about_a_pick_later_lands_on_that_pick_whatever_order_it_comes_in() {
        use crate::element::Heading;
        let mut r = Round::default();
        r.push(paid(7));
        r.push(paid(8));
        // The second pick's answer comes back first.
        assert!(r.locate(8, Some((3, 4)), Some(Heading { text: "Invoices".into(), inside: false })));
        assert_eq!(r.reference(None), "[Clipframes: 2 things in Google Chrome \"Invoices\"]\n1. Text \"Paid\"\n2. Text \"Paid\", 3rd of 4 on the page, under heading \"Invoices\"");
        assert!(r.locate(7, Some((1, 4)), None));
        assert_eq!(r.reference(None), "[Clipframes: 2 things in Google Chrome \"Invoices\"]\n1. Text \"Paid\", 1st of 4 on the page\n2. Text \"Paid\", 3rd of 4 on the page, under heading \"Invoices\"");
    }

    #[test]
    fn what_is_learned_about_a_pick_that_was_removed_meanwhile_goes_nowhere() {
        let mut r = Round::default();
        r.push(paid(7));
        r.push(paid(8));
        r.remove(0);
        assert!(!r.locate(7, Some((1, 4)), None), "the pick is gone");
        assert_eq!(r.reference(None), "[Text \"Paid\" in Google Chrome \"Invoices\"]", "and the one that moved into its place is not touched");
        // The one that moved up is still found, by what it is known by and not by where it is.
        assert!(r.locate(8, Some((2, 4)), None));
        assert_eq!(r.reference(None), "[Text \"Paid\", 2nd of 4 on the page in Google Chrome \"Invoices\"]");
    }

    #[test]
    fn what_is_learned_for_a_round_that_is_over_does_not_reach_the_next_one() {
        // A new round is a new `Round`; what the old one's picks were known by is not in it,
        // and picks made without a look to come (areas, clips, old captures) are known by nothing.
        let mut next = Round::default();
        next.add(button());
        next.push(paid(9));
        assert!(!next.locate(7, Some((1, 4)), None));
        assert!(!next.locate(0, Some((1, 4)), None), "zero is no pick's name");
        assert_eq!(next.picks[0].element.occurrence, None);
    }

    #[test]
    fn what_a_pick_is_known_by_is_not_written_to_disk() {
        let mut r = Round::default();
        r.push(paid(7));
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("\"id\""), "{json}");
        assert_eq!(serde_json::from_str::<Round>(&json).unwrap().picks[0].id, 0);
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
