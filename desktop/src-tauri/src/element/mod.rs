//! What the pointer is over: one model for every system, one back end per system.
//!
//! The fields and the wording built from them match the Swift app (app/Sources/Model.swift),
//! because the reference line and notes.md are the product's contract with the agent.

use serde::{Deserialize, Serialize};

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
    fn an_unnamed_element_falls_back_to_its_role() {
        assert_eq!(ElementInfo::default().headline(), "Element");
        assert_eq!(ElementInfo::default().selector(), "");
    }
}
