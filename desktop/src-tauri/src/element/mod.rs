//! What the pointer is over: one model for every system, one back end per system.
//!
//! The fields and the wording built from them match the Swift app (app/Sources/Model.swift),
//! because the reference line and notes.md are the product's contract with the agent.

use serde::{Deserialize, Serialize};

pub mod locate;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as platform;

#[cfg(not(any(target_os = "macos", windows)))]
mod linux;
#[cfg(not(any(target_os = "macos", windows)))]
use linux as platform;

/// A rectangle in global screen points, top-left origin.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ElementInfo {
    pub app: String,
    pub pid: i32,
    pub window: String,
    pub url: String,
    /// "Button", "Link", "TextField"… without the system's own prefix.
    pub role: String,
    pub name: String,
    pub value: String,
    /// The app's own identifier for the element (AXIdentifier, AutomationId).
    pub identifier: String,
    /// In web content: the element's id attribute.
    pub dom_id: String,
    /// In web content: its class list, space-separated.
    pub dom_classes: String,
    /// For an element without a name: the first text found inside it.
    pub inner_text: String,
    /// Up to four named containers around it, outermost first.
    pub path: Vec<String>,
    pub frame: Rect,
    /// When other elements in the same page or window have its role and name: which of them
    /// it is, counted from 1 in document order, and how many there are.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrence: Option<(u32, u32)>,
    /// The heading it comes under.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<Heading>,
}

/// The heading that says where on the page an element is.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Heading {
    pub text: String,
    /// False: the last heading before the element. True: there is none before it and this
    /// one is inside it, as in a section that begins with its own heading.
    pub inside: bool,
}

impl ElementInfo {
    pub fn role_name(&self) -> &str {
        if self.role.is_empty() { "Element" } else { &self.role }
    }

    /// `Button "New invoice"`, as the reference line and notes.md say it.
    pub fn headline(&self) -> String {
        if !self.name.is_empty() {
            format!("{} \"{}\"", self.role_name(), self.name)
        } else if !self.inner_text.is_empty() {
            format!("{} containing \"{}\"", self.role_name(), self.inner_text)
        } else if !self.value.is_empty() {
            format!("{} \"{}\"", self.role_name(), self.value)
        } else {
            self.role_name().to_string()
        }
    }

    /// `#new-invoice .btn.btn-primary`: what an agent can search the code for.
    pub fn selector(&self) -> String {
        let mut parts = Vec::new();
        if !self.dom_id.is_empty() {
            parts.push(format!("#{}", self.dom_id));
        }
        let classes: Vec<&str> = self.dom_classes.split_whitespace().take(6).collect();
        if !classes.is_empty() {
            parts.push(format!(".{}", classes.join(".")));
        }
        if !self.identifier.is_empty() {
            parts.push(format!("id={}", self.identifier));
        }
        parts.join(" ")
    }

    /// `2nd of 2 on the page`: which of several elements that read the same this one is.
    /// "On the page" inside a browser or a web view, "in the window" in any other app.
    pub fn which_one(&self) -> Option<String> {
        let (nth, of) = self.occurrence.filter(|(nth, of)| *of > 1 && *nth >= 1 && nth <= of)?;
        Some(format!("{} of {of} {}", locate::ordinal(nth), if self.url.is_empty() { "in the window" } else { "on the page" }))
    }

    /// `under heading "Try it on your own app."`, or `with heading "Questions"` for
    /// something that holds its heading itself.
    pub fn under_what(&self) -> Option<String> {
        let heading = self.heading.as_ref().filter(|h| !h.text.is_empty())?;
        Some(format!("{} heading \"{}\"", if heading.inside { "with" } else { "under" }, heading.text))
    }

    /// Both of those, in the order they are said. The one place that words them, for the
    /// pasted line and for notes.md.
    pub fn whereabouts(&self) -> Vec<String> {
        self.which_one().into_iter().chain(self.under_what()).collect()
    }
}

/// Why nothing could be read.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum ReadError {
    /// The user has not allowed this app to read other apps (macOS Accessibility).
    NotPermitted,
    /// Not built for this system yet.
    Unsupported(String),
    /// The system gave no element there.
    Nothing,
    Other(String),
}

/// Whether this app may read other apps' elements. Always true where no permission is needed.
pub fn permitted() -> bool {
    platform::permitted()
}

/// Has the system ask the user for that permission, where there is one to ask for.
pub fn ask_permission() {
    platform::ask_permission()
}

/// The pointer's position in global screen points, top-left origin.
pub fn pointer() -> Option<(f64, f64)> {
    platform::pointer()
}

/// The element a person means at a point on screen, in another app's window.
pub fn element_at(x: f64, y: f64) -> Result<ElementInfo, ReadError> {
    platform::element_at(x, y)
}

/// The window the user is working in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Foreground {
    pub app: String,
    pub title: String,
    pub pid: i32,
    pub frame: Rect,
}

/// The frontmost window of another app, if there is one. Cheap: it is asked once a second.
pub fn foreground() -> Option<Foreground> {
    platform::foreground()
}

/// The same, with everything worth knowing about it: what it sits inside, the page it is on.
/// Slower where that takes extra questions, so it is asked once, when the user clicks.
pub fn element_full_at(x: f64, y: f64) -> Result<ElementInfo, ReadError> {
    platform::element_full_at(x, y)
}

/// The address of the page shown at a point, for knowing which site is in front. Chromium and
/// Electron apps on macOS only have one to give while their page structure is switched on,
/// which costs them CPU and memory for as long as it stays on: `may_wake` says whether this
/// reading is worth switching it on for.
pub fn page_at(x: f64, y: f64, may_wake: bool) -> String {
    platform::page_at(x, y, may_wake)
}

/// The full reading of an element the user just clicked: everything `element_full_at` says,
/// and which one it is among those that read the same and what heading it is under. That
/// means going through the page or window, within `locate::BUDGET`. Never for a hover.
pub fn element_picked_at(x: f64, y: f64) -> Result<ElementInfo, ReadError> {
    platform::element_picked_at(x, y)
}

/// Housekeeping to call now and then: lets apps that were asked for their page structure go
/// back to sleep once they have not been looked at for `older_than`.
pub fn sleep_idle(older_than: std::time::Duration) {
    platform::sleep_idle(older_than)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headline_and_selector_match_the_swift_app() {
        let e = ElementInfo {
            role: "Button".into(),
            name: "New invoice".into(),
            dom_id: "new-invoice".into(),
            dom_classes: "btn btn-primary".into(),
            ..Default::default()
        };
        assert_eq!(e.headline(), "Button \"New invoice\"");
        assert_eq!(e.selector(), "#new-invoice .btn.btn-primary");
    }

    #[test]
    fn which_one_and_under_what_are_worded_in_one_place() {
        let mut e = ElementInfo { role: "Link".into(), name: "Download".into(), url: "https://clipframes.com/".into(), ..Default::default() };
        assert_eq!(e.whereabouts(), Vec::<String>::new());
        e.occurrence = Some((2, 2));
        e.heading = Some(Heading { text: "Try it on your own app.".into(), inside: false });
        assert_eq!(e.whereabouts(), ["2nd of 2 on the page", "under heading \"Try it on your own app.\""]);
        // A native app has windows, not pages; a section can hold its own heading.
        e.url.clear();
        e.occurrence = Some((11, 23));
        e.heading = Some(Heading { text: "Questions".into(), inside: true });
        assert_eq!(e.whereabouts(), ["11th of 23 in the window", "with heading \"Questions\""]);
        // The only one of its kind is not numbered, whatever was stored.
        e.occurrence = Some((1, 1));
        e.heading = None;
        assert_eq!(e.whereabouts(), Vec::<String>::new());
    }

    #[test]
    fn captures_from_before_these_facts_still_load_and_new_ones_only_add_them_when_known() {
        let old = r#"{"app":"Google Chrome","role":"Button","name":"Save","path":[],"frame":{"x":1,"y":2,"width":3,"height":4}}"#;
        let e: ElementInfo = serde_json::from_str(old).unwrap();
        assert_eq!((e.occurrence, e.heading.clone()), (None, None));
        assert!(!serde_json::to_string(&e).unwrap().contains("occurrence"));
        let new = ElementInfo { occurrence: Some((2, 4)), heading: Some(Heading { text: "Invoices".into(), inside: false }), ..e };
        let json = serde_json::to_string(&new).unwrap();
        assert!(json.contains(r#""occurrence":[2,4],"heading":{"text":"Invoices","inside":false}"#), "{json}");
        assert_eq!(serde_json::from_str::<ElementInfo>(&json).unwrap(), new);
    }

    #[test]
    fn an_unnamed_element_falls_back_to_its_role() {
        assert_eq!(ElementInfo::default().headline(), "Element");
        assert_eq!(ElementInfo::default().selector(), "");
    }
}
