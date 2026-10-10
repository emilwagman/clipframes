//! Where Clipframes has been used: an app, or a site inside a browser. When one of those
//! comes to the front again, a small tab appears there, so the tool is seen where it is useful.
//!
//! Using Clipframes somewhere remembers the place. The user can turn the tab off for a place,
//! and it stays off there until they turn it on again: using Clipframes there does not.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

/// A place is remembered this long after it was last used.
pub const RECENT: u64 = 30 * 24 * 60 * 60;
const FILE: &str = "places.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Place {
    pub app: String,
    /// The site, for a page in a browser: "localhost:3000", "github.com". Empty otherwise.
    pub host: String,
}

impl Place {
    pub fn new(app: &str, url: &str) -> Place {
        Place { app: app.trim().to_string(), host: host_of(url) }
    }

    pub fn known(&self) -> bool {
        !self.app.is_empty()
    }

    fn key(&self) -> String {
        if self.host.is_empty() { self.app.clone() } else { format!("{}|{}", self.app, self.host) }
    }

    fn of_key(key: &str) -> Place {
        let (app, host) = key.split_once('|').unwrap_or((key, ""));
        Place { app: app.into(), host: host.into() }
    }

    /// "Google Chrome · localhost:3000", "Slack".
    pub fn name(&self) -> String {
        if self.host.is_empty() { self.app.clone() } else { format!("{} · {}", self.app, self.host) }
    }
}

/// "localhost:3000" from "http://localhost:3000/invoices?x=1". A file opened in a browser has
/// no site, so every local file counts as one place: "local file".
pub fn host_of(url: &str) -> String {
    let url = url.trim();
    if url.starts_with("file:") {
        return "local file".into();
    }
    let rest = match url.split_once("://") {
        Some((scheme, rest)) if scheme == "http" || scheme == "https" => rest,
        // An address bar may show the address without its scheme.
        None if !url.is_empty() && !url.contains(' ') && url.contains('.') | url.starts_with("localhost") => url,
        _ => return String::new(),
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or(host);
    host.strip_prefix("www.").unwrap_or(host).to_ascii_lowercase()
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Entry {
    /// Seconds since 1970.
    last_used: u64,
    /// Whether the tab shows by itself here.
    auto: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Places {
    places: HashMap<String, Entry>,
}

impl Places {
    pub fn load(dir: &Path) -> Places {
        fs::read(dir.join(FILE)).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        let part = dir.join("places.part");
        fs::write(&part, serde_json::to_vec_pretty(self).map_err(io::Error::other)?)?;
        fs::rename(part, dir.join(FILE))
    }

    /// Clipframes was used here: remember it. A place seen for the first time gets the tab;
    /// one that is known keeps its switch, so a tab that was turned off stays off.
    pub fn used(&mut self, place: &Place, now: u64) {
        if place.known() {
            self.places.entry(place.key()).or_insert(Entry { last_used: now, auto: true }).last_used = now;
        }
        // Places not used for a long time are forgotten, so the file never grows without end.
        // Not the ones where the tab was turned off: forgetting those would turn it on again.
        self.places.retain(|_, e| !e.auto || now.saturating_sub(e.last_used) < RECENT * 6);
    }

    pub fn set_auto(&mut self, place: &Place, on: bool, now: u64) {
        if place.known() {
            let entry = self.places.entry(place.key()).or_insert(Entry { last_used: now, auto: on });
            entry.auto = on;
            // Asking for the tab here counts as using Clipframes here: the tab is the only
            // thing that was asked for, and it must show even if nothing was picked lately.
            if on {
                entry.last_used = now;
            }
        }
    }

    /// Whether the tab is on for this place: what the pin in the bar shows. A place not seen
    /// yet has it off until something is picked there.
    pub fn auto(&self, place: &Place) -> bool {
        self.places.get(&place.key()).is_some_and(|e| e.auto)
    }

    /// Whether the tab could appear on some site inside this app. Only then is it worth
    /// finding out which site the app is showing.
    pub fn has_site(&self, app: &str, now: u64) -> bool {
        let sites = format!("{}|", app.trim());
        self.places.iter().any(|(key, e)| key.starts_with(&sites) && e.auto && now.saturating_sub(e.last_used) < RECENT)
    }

    /// Whether the tab should appear now that this place is in front.
    pub fn wants(&self, place: &Place, now: u64) -> bool {
        self.places.get(&place.key()).is_some_and(|e| e.auto && now.saturating_sub(e.last_used) < RECENT)
    }

    /// The places the tab appears in now, the one used last first: the list in Settings.
    pub fn shown(&self, now: u64) -> Vec<Place> {
        let mut shown: Vec<(&String, &Entry)> = self.places.iter().filter(|(_, e)| e.auto && now.saturating_sub(e.last_used) < RECENT).collect();
        shown.sort_by(|a, b| b.1.last_used.cmp(&a.1.last_used).then(a.0.cmp(b.0)));
        shown.into_iter().map(|(key, _)| Place::of_key(key)).collect()
    }
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 24 * 60 * 60;

    #[test]
    fn a_site_is_its_host_and_port() {
        assert_eq!(host_of("http://localhost:3000/invoices?tab=2"), "localhost:3000");
        assert_eq!(host_of("https://www.github.com/emilwagman/clipframes"), "github.com");
        assert_eq!(host_of("localhost:5199"), "localhost:5199");
        assert_eq!(host_of("github.com/pulls"), "github.com");
        assert_eq!(host_of("file:///C:/site/index.html"), "local file");
        assert_eq!(host_of(""), "");
        assert_eq!(host_of("chrome://settings"), "");
        assert_eq!(host_of("Search or type a URL"), "");
    }

    #[test]
    fn using_clipframes_somewhere_makes_the_tab_appear_there() {
        let mut places = Places::default();
        let app = Place::new("Google Chrome", "http://localhost:3000/");
        assert!(!places.wants(&app, 1000));
        places.used(&app, 1000);
        assert!(places.wants(&app, 1000 + DAY));
        assert!(!places.wants(&Place::new("Google Chrome", "https://github.com/"), 1000), "another site in the same browser is another place");
        assert!(!places.wants(&Place::new("Slack", ""), 1000));
    }

    #[test]
    fn only_an_app_with_a_site_the_tab_appears_on_has_its_pages_read() {
        let mut places = Places::default();
        places.used(&Place::new("Slack", ""), 0);
        assert!(!places.has_site("Slack", DAY), "used as an app, never on a page");
        assert!(!places.has_site("Google Chrome", DAY));
        let site = Place::new("Google Chrome", "http://localhost:3000/");
        places.used(&site, 0);
        assert!(places.has_site("Google Chrome", DAY));
        assert!(!places.has_site("Google Chrome Canary", DAY));
        assert!(!places.has_site("Google Chrome", 31 * DAY), "not used there for a month");
        places.set_auto(&site, false, DAY);
        assert!(!places.has_site("Google Chrome", 2 * DAY), "the tab was turned off there");
    }

    #[test]
    fn a_place_not_used_for_a_month_stops_showing_the_tab() {
        let mut places = Places::default();
        let slack = Place::new("Slack", "");
        places.used(&slack, 0);
        assert!(places.wants(&slack, 29 * DAY));
        assert!(!places.wants(&slack, 31 * DAY));
    }

    #[test]
    fn turning_it_off_holds_until_the_user_turns_it_on_again() {
        let mut places = Places::default();
        let slack = Place::new("Slack", "");
        places.used(&slack, 0);
        places.set_auto(&slack, false, DAY);
        assert!(!places.wants(&slack, 2 * DAY));
        assert!(!places.auto(&slack));
        places.used(&slack, 3 * DAY);
        assert!(!places.wants(&slack, 3 * DAY), "picking something there does not turn it back on");
        assert!(!places.auto(&slack));
        places.set_auto(&slack, true, 4 * DAY);
        assert!(places.wants(&slack, 4 * DAY), "the pin does");
        assert!(!places.wants(&slack, 4 * DAY + 31 * DAY), "and from then it is a place like any other");
        // Turned on long after the last pick there, it shows at once.
        places.set_auto(&slack, false, 5 * DAY);
        places.set_auto(&slack, true, 90 * DAY);
        assert!(places.wants(&slack, 91 * DAY));
    }

    #[test]
    fn a_tab_turned_off_before_anything_was_picked_there_stays_off() {
        let mut places = Places::default();
        let site = Place::new("Google Chrome", "http://localhost:3000/");
        places.set_auto(&site, false, 0);
        places.used(&site, DAY);
        assert!(!places.wants(&site, DAY));
    }

    #[test]
    fn a_place_where_the_tab_is_off_is_never_forgotten() {
        let mut places = Places::default();
        let (slack, figma) = (Place::new("Slack", ""), Place::new("Figma", ""));
        places.used(&slack, 0);
        places.used(&figma, 0);
        places.set_auto(&slack, false, 0);
        // A year on, something is picked somewhere else.
        places.used(&Place::new("Notes", ""), 365 * DAY);
        places.used(&slack, 366 * DAY);
        places.used(&figma, 366 * DAY);
        assert!(!places.wants(&slack, 366 * DAY), "still off");
        assert!(places.wants(&figma, 366 * DAY), "forgotten, so it is a new place again");
    }

    #[test]
    fn settings_lists_the_places_the_tab_appears_in_newest_first() {
        let mut places = Places::default();
        let (slack, site, old, off) = (Place::new("Slack", ""), Place::new("Google Chrome", "http://localhost:3000/"), Place::new("Figma", ""), Place::new("Notes", ""));
        places.used(&old, 0);
        places.used(&slack, 40 * DAY);
        places.used(&site, 41 * DAY);
        places.used(&off, 41 * DAY);
        places.set_auto(&off, false, 41 * DAY);
        assert_eq!(places.shown(42 * DAY), vec![site.clone(), slack], "not the one unused for a month, not the one turned off");
        assert_eq!(places.shown(42 * DAY)[0].name(), "Google Chrome · localhost:3000");
    }

    #[test]
    fn places_survive_a_restart() {
        let dir = std::env::temp_dir().join(format!("clipframes-places-{}", std::process::id()));
        let mut places = Places::default();
        places.used(&Place::new("Slack", ""), 5);
        places.save(&dir).unwrap();
        assert_eq!(Places::load(&dir), places);
        assert_eq!(Places::load(&dir.join("missing")), Places::default());
        fs::remove_dir_all(&dir).unwrap();
    }
}
