//! The app: a tray icon, one shortcut, the bar, and a round of picks.
//!
//! Nothing is open while Clipframes is idle. A web view costs a few hundred megabytes on
//! Windows even when hidden, so every window is built when the bar opens. The overlays, the
//! comment box and the input source go away when it closes; the bar stays hidden for a short
//! while, so opening it again straight away is instant, and then it goes too.
//!
//! Positions come in "picker units": what the system's input and element APIs report. That is
//! points on macOS and physical pixels on Windows, so every conversion to a window lives here.

use crate::claude;
use crate::element::{self, ElementInfo, Rect};
use crate::picker::{Event, Mode, Picker};
use crate::places::{self, Place, Places};
use crate::round::{Click, Kind, Pick, Round};
use crate::shot;
use crate::tab;
use crate::telemetry;
use crate::settings::{self, Settings};
use crate::store::{self, Stamp};
use crate::updates;
use serde::Serialize;
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg(all(feature = "selftest", target_os = "macos"))]
#[path = "selftest.rs"]
mod selftest;

const BAR: &str = "bar";
const NOTE: &str = "note";
const SETTINGS: &str = "settings";
const HISTORY: &str = "history";
/// Overlays are "overlay-0", "overlay-1", …: one per display.
const OVERLAY: &str = "overlay-";
#[cfg(not(windows))]
const TAB: &str = "tab";
/// A clip stops by itself after this long.
const LONGEST_CLIP: Duration = Duration::from_secs(60);
/// Clip frames are taken this far apart and no wider than this.
const FRAME_EVERY: Duration = Duration::from_millis(250);
const FRAME_WIDTH: u32 = 1600;
/// Pictures are shrunk to these widths at most: enough to read, without filling the disk.
const ELEMENT_WIDTH: u32 = 1600;
const AREA_WIDTH: u32 = 2560;
/// How long the hidden bar is kept after closing, ready to open again at once.
const KEEP_WARM: Duration = Duration::from_secs(90);
/// How far the clipboard and the files on disk may trail behind typing in the comment box.
const NOTE_SETTLE: Duration = Duration::from_millis(200);
/// The bar has come to rest when it has not moved for this long.
const BAR_SETTLE: Duration = Duration::from_millis(120);

/// Sizes in CSS pixels.
const BAR_SIZE: (f64, f64) = (404.0, 64.0);
const NOTE_SIZE: (f64, f64) = (316.0, 172.0);
/// The one-line comment box: its height with one line, and with as many as it grows to.
const LINE_HEIGHT: (f64, f64) = (60.0, 100.0);

/// Where the tab sits. Ways being compared; chosen with CLIPFRAMES_TAB_PLACE at start.
#[derive(Debug, Clone, Copy, PartialEq)]
enum TabPlace {
    /// In the middle of where the bar would open.
    Bar,
    /// Bottom centre of the display, where the bar opens when it was never moved.
    Edge,
    /// At that height, straight under (or over) where the bar would open.
    Under,
    /// Against the edge of the display nearest to where the bar would open, half out of sight.
    Nearest,
}

fn tab_place_named(name: &str) -> TabPlace {
    match name.trim().to_ascii_lowercase().as_str() {
        "edge" => TabPlace::Edge,
        "under" => TabPlace::Under,
        "nearest" => TabPlace::Nearest,
        _ => TabPlace::Bar,
    }
}

fn tab_place() -> TabPlace {
    static PLACE: std::sync::OnceLock<TabPlace> = std::sync::OnceLock::new();
    *PLACE.get_or_init(|| tab_place_named(&std::env::var("CLIPFRAMES_TAB_PLACE").unwrap_or_default()))
}

/// The tab's top-left corner. Everything is in the display's own pixels: the display, the
/// part of it the system's bars leave free, where the bar would open, where it opens when it
/// was never moved, and the tab's side.
fn tab_at(place: TabPlace, display: &Rect, area: &Rect, bar: &Rect, home: &Rect, side: f64) -> (f64, f64) {
    let middle = |r: &Rect| (r.x + r.width / 2.0, r.y + r.height / 2.0);
    // Along an edge the tab stays whole inside the free part of the display.
    let along = |at: f64, from: f64, length: f64| (at - side / 2.0).clamp(from, (from + length - side).max(from));
    let (bar, home) = (middle(bar), middle(home));
    match place {
        TabPlace::Bar => (bar.0 - side / 2.0, bar.1 - side / 2.0),
        TabPlace::Edge => (home.0 - side / 2.0, home.1 - side / 2.0),
        TabPlace::Under => (along(bar.0, area.x, area.width), home.1 - side / 2.0),
        TabPlace::Nearest => {
            // Half of it past the edge of the display. Where one of the system's bars lies
            // along that edge (menu bar, Dock, taskbar) it stays whole, against that bar.
            let across = |edge: f64, end: f64, outward: f64| if (edge - end).abs() < 1.0 { edge - side / 2.0 } else { edge - side / 2.0 - outward * side / 2.0 };
            let (right, bottom) = (area.x + area.width, area.y + area.height);
            let edges = [
                (bottom - bar.1, (along(bar.0, area.x, area.width), across(bottom, display.y + display.height, 1.0))),
                (bar.1 - area.y, (along(bar.0, area.x, area.width), across(area.y, display.y, -1.0))),
                (bar.0 - area.x, (across(area.x, display.x, -1.0), along(bar.1, area.y, area.height))),
                (right - bar.0, (across(right, display.x + display.width, 1.0), along(bar.1, area.y, area.height))),
            ];
            // The bottom edge first, so a bar that was never moved keeps its tab below it.
            edges.into_iter().reduce(|nearest, edge| if edge.0 < nearest.0 { edge } else { nearest }).map(|(_, at)| at).unwrap_or(home)
        }
    }
}

/// How the comment box behaves. Ways being compared; chosen with CLIPFRAMES_NOTE_STYLE at start.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum NoteStyle {
    /// Under the pick, three lines and two buttons.
    Box,
    /// Under the pick, one line tall, growing as more is typed.
    Line,
    /// The box, beside the pick where there is room: right, left, below, above.
    Aside,
    /// The box under the pick, which steps to another side when the pointer comes for it.
    Yield,
}

fn note_style_named(name: &str) -> NoteStyle {
    match name.trim().to_ascii_lowercase().as_str() {
        "line" => NoteStyle::Line,
        "aside" => NoteStyle::Aside,
        "yield" => NoteStyle::Yield,
        _ => NoteStyle::Box,
    }
}

fn note_style() -> NoteStyle {
    static STYLE: std::sync::OnceLock<NoteStyle> = std::sync::OnceLock::new();
    *STYLE.get_or_init(|| note_style_named(&std::env::var("CLIPFRAMES_NOTE_STYLE").unwrap_or_default()))
}

/// The comment box's top-left corner for a pick, on the display `screen`, all in the same
/// units. `gap` is kept between the box and the pick, and between the box and the display's
/// edges. `stepped` is the yielding box after it has stepped out of the pointer's way.
fn note_spot(style: NoteStyle, screen: &Rect, pick: &Rect, size: (f64, f64), gap: f64, stepped: bool) -> (f64, f64) {
    let (w, h) = size;
    let (left, top) = (screen.x + gap, screen.y + gap);
    let (right, bottom) = ((screen.x + screen.width - w - gap).max(left), (screen.y + screen.height - h - gap).max(top));
    let fits = |at: &(f64, f64)| at.0 >= left && at.0 <= right && at.1 >= top && at.1 <= bottom;
    // Under or over the pick it lines up with the pick's left edge, beside it with its top.
    let (x, y) = (pick.x.clamp(left, right), pick.y.clamp(top, bottom));
    let below = (x, pick.y + pick.height + gap);
    let above = (x, pick.y - h - gap);
    let beside = [(pick.x + pick.width + gap, y), (pick.x - w - gap, y)];
    // Under the pick, or over it when there is no room under. With room on neither side
    // (a pick as tall as the display) it ends at the top of the display, on the pick itself.
    let usual = if below.1 <= bottom { below } else { (x, above.1.clamp(top, bottom)) };
    match style {
        NoteStyle::Aside => beside.into_iter().chain([below, above]).find(fits).unwrap_or(usual),
        NoteStyle::Yield if stepped => [above, beside[0], beside[1]].into_iter().find(|at| fits(at) && *at != usual).unwrap_or(usual),
        _ => usual,
    }
}

/// How far a point is from a rectangle: nothing when it is inside.
fn away(r: &Rect, x: f64, y: f64) -> f64 {
    let (dx, dy) = ((r.x - x).max(x - (r.x + r.width)).max(0.0), (r.y - y).max(y - (r.y + r.height)).max(0.0));
    dx.hypot(dy)
}

/// Whether the pointer has come for what the yielding comment box covers: it is close to
/// the box now, and has come a good way closer than it was (`farthest`) since the box
/// opened. A pointer that only wobbles on the pick it just clicked does not count.
fn comes_for(farthest: f64, now: f64) -> bool {
    now <= 24.0 && farthest - now >= 12.0
}

/// A line on stderr with the time since start, when CLIPFRAMES_TRACE is set. For chasing
/// the order of things across threads on a machine with no debugger.
fn trace(what: &str) {
    static START: std::sync::OnceLock<(Instant, bool)> = std::sync::OnceLock::new();
    let (start, on) = START.get_or_init(|| (Instant::now(), std::env::var_os("CLIPFRAMES_TRACE").is_some()));
    if *on {
        eprintln!("{:>9.1} ms  [{:?}] {what}", start.elapsed().as_secs_f64() * 1000.0, thread::current().id());
    }
}

/// One display and the overlay window that covers it.
#[derive(Debug, Clone)]
struct Screen {
    label: String,
    /// The display in picker units.
    frame: Rect,
    /// CSS pixels per picker unit on this display.
    css: f64,
}

impl Screen {
    /// A global rectangle in this overlay's own CSS pixels.
    fn local(&self, r: &Rect) -> Rect {
        Rect { x: (r.x - self.frame.x) * self.css, y: (r.y - self.frame.y) * self.css, width: r.width * self.css, height: r.height * self.css }
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.frame.x && x < self.frame.x + self.frame.width && y >= self.frame.y && y < self.frame.y + self.frame.height
    }
}

#[derive(Default)]
pub struct Core {
    /// Held for the whole of an open and of a close, so the two never run into each other:
    /// every trigger (shortcut, Esc, tray, tab, a second launch) arrives on its own thread.
    /// Never taken on the main thread, which the holder waits for while it builds windows.
    gate: Mutex<()>,
    round: Mutex<Round>,
    picker: Mutex<Option<Picker>>,
    screens: Mutex<Vec<Screen>>,
    /// The pick whose comment box is open.
    noting: Mutex<Option<usize>>,
    /// The highlight the overlays are showing, so an unchanged answer is not sent again.
    shown: Mutex<Option<Rect>>,
    /// Kept for the whole run: on Linux the clipboard's content lives only as long as its owner.
    clipboard: Mutex<Option<arboard::Clipboard>>,
    /// What Clipframes last put on the clipboard.
    copied: Mutex<String>,
    /// The round's folder and when it began, once something has been picked.
    folder: Mutex<Option<(PathBuf, Stamp)>>,
    /// The round's notes.md, once written: the path the pasted reference points to.
    notes: Mutex<Option<PathBuf>>,
    places: Mutex<Places>,
    /// The app or site in front right now, as far as it is known.
    front: Mutex<Option<Place>>,
    /// The app or site the bar was opened over.
    over: Mutex<Option<Place>>,
    #[cfg(windows)]
    tab: Mutex<Option<tab::Tab>>,
    /// The tool that is on in the bar.
    tool: Mutex<Kind>,
    /// The clip being recorded.
    recording: Mutex<Option<Recording>>,
    /// Numbers handed to this round's pictures, so a removed pick never frees a name.
    files: AtomicU32,
    settings: Mutex<Settings>,
    /// False when another app owns the shortcut, so Clipframes could not take it.
    shortcut_works: Mutex<bool>,
    /// The tray's "Open" line, which shows the shortcut.
    open_item: Mutex<Option<MenuItem<tauri::Wry>>>,
    /// The last thing the updater had to say.
    update: Mutex<String>,
    /// When the round on screen was opened, for the report of how long it stayed.
    began: Mutex<Option<Instant>>,
    /// Why the round could not start, for a bar that loads after the fact.
    trouble: Mutex<Option<String>>,
    /// "Open Settings" was pressed for Screen Recording since the app started.
    asked_screen: AtomicBool,
    /// Counts opens and closes, so a keep-warm timer knows if it is out of date.
    turn: AtomicU64,
    /// When the bar was asked to open, to time how long until it can draw.
    opened: Mutex<Option<Instant>>,
    /// A comment was typed that the clipboard and the round's folder do not have yet.
    note_unsaved: AtomicBool,
    /// A thread is waiting to write that comment out.
    note_timer: AtomicBool,
    /// The number that came with the newest comment text, so an older one arriving late is
    /// not put over it.
    note_seq: AtomicU64,
    /// The place watcher's thread is running (`watch`).
    watching: AtomicBool,
    /// Set while the bar is where the user's hand may move it: from a press on its grip until
    /// the bar is next opened, closed or sent home. Holds which displays were connected at the
    /// press. Without it, Clipframes putting the bar somewhere would look like a drag.
    gripped: Mutex<Option<String>>,
    /// The bar has moved, or its grip was pressed, since it was last looked at.
    bar_moved: AtomicBool,
    /// It really moved, so there is a new place to remember.
    bar_dragged: AtomicBool,
    /// A thread is waiting for the bar to come to rest.
    bar_timer: AtomicBool,
    /// What the open comment box is for and where it is.
    noted: Mutex<Option<Noted>>,
}

/// The comment box on screen.
#[derive(Debug, Clone)]
struct Noted {
    /// The pick it belongs to.
    pick: Rect,
    /// The box itself, in picker units.
    at: Rect,
    /// Its window's height in CSS pixels: the one-line box grows.
    height: f64,
    /// The farthest the pointer has been from it since it opened, and whether it has
    /// stepped out of the pointer's way: the yielding box does that once for a pick.
    farthest: f64,
    stepped: bool,
}

/// A clip in the making.
#[derive(Clone)]
struct Recording {
    started: Instant,
    stop: Arc<AtomicBool>,
    /// Seconds in, and what was clicked.
    clicks: Arc<Mutex<Vec<(f64, String)>>>,
}

/// What the bar and the comment box draw.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoundView {
    picking: bool,
    tool: Kind,
    /// Seconds recorded so far, while a clip is being recorded.
    recording: Option<f64>,
    /// The app or site the bar is open over.
    place: Option<PlaceView>,
    picks: Vec<PickView>,
    /// The pick whose comment box is open.
    noting: Option<usize>,
    /// What is on the clipboard now.
    reference: String,
    /// Set when the round could not start, e.g. the system has not allowed Clipframes yet.
    trouble: Option<String>,
    /// The shortcut as people write it, e.g. "Ctrl+Shift+Space".
    shortcut: String,
    /// Which comment box to draw.
    note_style: NoteStyle,
}

#[derive(Debug, Clone, Serialize)]
struct PlaceView {
    name: String,
    auto: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PickView {
    kind: Kind,
    headline: String,
    selector: String,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct HoverView {
    rect: Option<Rect>,
    label: String,
}

#[derive(Debug, Clone, Serialize)]
struct MarkView {
    number: usize,
    rect: Rect,
    kind: Kind,
}

#[derive(Debug, Clone, Serialize)]
struct AreaView {
    rect: Option<Rect>,
    recording: bool,
}

fn view(app: &AppHandle) -> RoundView {
    let core = app.state::<Core>();
    let round = core.round.lock().unwrap();
    let picking = core.picker.lock().unwrap().is_some();
    let noting = *core.noting.lock().unwrap();
    let trouble = core.trouble.lock().unwrap().clone();
    let recording = core.recording.lock().unwrap().as_ref().map(|r| r.started.elapsed().as_secs_f64());
    let state = RoundView {
        picking,
        tool: *core.tool.lock().unwrap(),
        recording,
        // With the tab switched off everywhere there is nothing for the pin to turn on.
        place: core.over.lock().unwrap().as_ref().filter(|p| p.known() && core.settings.lock().unwrap().show_tab).map(|p| PlaceView { name: p.name(), auto: core.places.lock().unwrap().auto(p) }),
        picks: round.picks.iter().map(|p| PickView { kind: p.kind, headline: p.headline(), selector: if p.kind == Kind::Element { p.element.selector() } else { String::new() }, note: p.note.clone() }).collect(),
        noting,
        reference: round.reference(core.notes.lock().unwrap().as_deref().and_then(|p| p.to_str())),
        trouble,
        shortcut: settings::label(&core.settings.lock().unwrap().shortcut, cfg!(target_os = "macos")),
        note_style: note_style(),
    };
    state
}

fn copy_text(app: &AppHandle, text: &str) {
    let core = app.state::<Core>();
    let mut clipboard = core.clipboard.lock().unwrap();
    if clipboard.is_none() {
        *clipboard = arboard::Clipboard::new().ok();
    }
    if let Some(c) = clipboard.as_mut() {
        // Windows programs expect CRLF; a terminal there joins lines that end in a bare LF.
        let _ = c.set_text(if cfg!(windows) { text.replace('\n', "\r\n") } else { text.to_string() });
        *core.copied.lock().unwrap() = text.to_string();
    }
}

/// Empties the clipboard if what is on it is still what Clipframes put there. Anything the
/// user has copied since is theirs and stays.
fn uncopy(app: &AppHandle) {
    if clipboard_is_ours(app) {
        copy_text(app, "");
    }
}

/// Whether the clipboard still holds what Clipframes last put there.
fn clipboard_is_ours(app: &AppHandle) -> bool {
    let core = app.state::<Core>();
    let ours = core.copied.lock().unwrap().clone();
    let now = core.clipboard.lock().unwrap().as_mut().and_then(|c| c.get_text().ok());
    now.is_some_and(|now| same_text(&now, &ours))
}

/// What to do with something learned about a pick after it was made.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Late {
    /// The round is open: everything is told and copied again, as after any change.
    Publish,
    /// The round is over and the clipboard still holds its text: the files and the clipboard
    /// get the fuller text.
    SaveAndCopy,
    /// The round is over and the user has copied something else since: only the files.
    Save,
}

fn late(round_open: bool, clipboard_ours: bool) -> Late {
    match (round_open, clipboard_ours) {
        (true, _) => Late::Publish,
        (false, true) => Late::SaveAndCopy,
        (false, false) => Late::Save,
    }
}

/// Whether two pieces of clipboard text are the same, however their lines end.
fn same_text(a: &str, b: &str) -> bool {
    !a.is_empty() && a.replace("\r\n", "\n") == b.replace("\r\n", "\n")
}

/// Tells every window what the round looks like now, and puts it on the clipboard.
fn publish(app: &AppHandle) {
    let core = app.state::<Core>();
    core.note_unsaved.store(false, Ordering::SeqCst);
    save(&core);
    let state = view(app);
    if !state.reference.is_empty() {
        copy_text(app, &state.reference);
    }
    let _ = app.emit("round", &state);

    let frames: Vec<(Rect, Kind)> = core.round.lock().unwrap().picks.iter().map(|p| (p.element.frame, p.kind)).collect();
    for screen in core.screens.lock().unwrap().iter() {
        let marks: Vec<MarkView> = frames.iter().enumerate().map(|(i, (f, kind))| MarkView { number: i + 1, rect: screen.local(f), kind: *kind }).collect();
        let _ = app.emit_to(screen.label.as_str(), "marks", marks);
    }
}

fn config_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok()
}

/// Clipframes was just used on this element's app or site: the tab will appear there.
fn remember(app: &AppHandle, element: &ElementInfo) {
    let core = app.state::<Core>();
    let place = Place::new(&element.app, &element.url);
    {
        let mut places = core.places.lock().unwrap();
        places.used(&place, places::now());
        if let Some(dir) = config_dir(app) {
            let _ = places.save(&dir);
        }
    }
    // The bar's switch is about the place being worked in now.
    *core.over.lock().unwrap() = Some(place);
}

/// Where the tab sits on the display that shows `near` (`tab_at`). `side` is the tab's side
/// in physical pixels of that display.
fn tab_spot(app: &AppHandle, near: (f64, f64), side: impl Fn(f64) -> f64) -> Option<Spot> {
    let shown = display_near(app, Some(near))?;
    let m = &shown.monitor;
    let (pos, size, scale) = (m.position(), m.size(), m.scale_factor());
    let display = Rect { x: pos.x as f64, y: pos.y as f64, width: size.width as f64, height: size.height as f64 };
    let (x, y) = tab_at(tab_place(), &display, &work_area(m), &bar_rect(app, &shown), &home_rect(m), side(scale));
    Some(Spot { at: PhysicalPosition::new(x as i32, y as i32), scale })
}

/// Shows the tab on the display that shows `near`: the middle of the window it belongs to.
#[cfg(windows)]
fn show_tab(app: &AppHandle, near: (f64, f64)) {
    let core = app.state::<Core>();
    let mut tab = core.tab.lock().unwrap();
    if tab.is_none() {
        let handle = app.clone();
        *tab = tab::Tab::start(protected(), move || {
            telemetry::via("tab");
            later(&handle, open)
        });
    }
    if let Some(tab) = tab.as_ref() {
        // The native tab keeps the size it was made at, whatever the display.
        let side = tab.side() as f64;
        if let Some(spot) = tab_spot(app, near, |_| side) {
            tab.show(spot.at.x, spot.at.y);
        }
    }
}

#[cfg(windows)]
fn hide_tab(app: &AppHandle) {
    if let Some(tab) = app.state::<Core>().tab.lock().unwrap().as_ref() {
        tab.hide();
    }
}

/// Elsewhere the tab is a very small window of the app's own.
#[cfg(not(windows))]
fn show_tab(app: &AppHandle, near: (f64, f64)) {
    let window = app.get_webview_window(TAB).or_else(|| small_window(app, TAB, (tab::SIZE + 12.0, tab::SIZE + 12.0)).build().ok());
    let Some(window) = window else { return };
    // A web view keeps its size in CSS pixels, so in pixels it is as large as its display says.
    if let Some(spot) = tab_spot(app, near, |scale| (tab::SIZE + 12.0) * scale) {
        spot.put(&window);
    }
    let _ = window.show();
}

#[cfg(not(windows))]
fn hide_tab(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(TAB) {
        let _ = window.hide();
    }
}

/// What the place watcher does with its turn.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Watch {
    /// The tab is switched off everywhere: nothing is left to watch for.
    Stop,
    /// A round is on: the tab is out of the way and no other app is asked anything.
    Wait,
    Look,
}

fn watch_turn(show_tab: bool, picking: bool) -> Watch {
    match (show_tab, picking) {
        (false, _) => Watch::Stop,
        (true, true) => Watch::Wait,
        (true, false) => Watch::Look,
    }
}

/// Starts the place watcher if the tab is switched on and it is not running already.
fn start_watch(app: &AppHandle) {
    let core = app.state::<Core>();
    if core.settings.lock().unwrap().show_tab && !core.watching.swap(true, Ordering::SeqCst) {
        let watcher = app.clone();
        if thread::Builder::new().name("clipframes-watch".into()).spawn(move || watch(watcher)).is_err() {
            core.watching.store(false, Ordering::SeqCst);
        }
    }
}

/// Once a second: which app or site is in front, and whether the tab belongs there. Reading
/// the page address is the only part that costs anything, and it happens only when the
/// window in front has changed. Runs for as long as the tab is switched on in Settings, and
/// ends within a second of it being switched off.
fn watch(app: AppHandle) {
    let mut seen: Option<(i32, String)> = None;
    let mut read_at = Instant::now();
    let mut showing = false;
    // The middle of the window the tab was last placed for.
    let mut placed = (f64::NAN, f64::NAN);
    loop {
        thread::sleep(Duration::from_secs(1));
        let core = app.state::<Core>();
        let picking = core.picker.lock().unwrap().is_some();
        let turn = {
            let settings = core.settings.lock().unwrap();
            let turn = watch_turn(settings.show_tab, picking);
            // Said while the setting is held, so switching the tab back on in this very
            // moment either finds the watcher still running or starts a new one.
            if turn == Watch::Stop {
                core.watching.store(false, Ordering::SeqCst);
            }
            turn
        };
        match turn {
            Watch::Stop => break,
            Watch::Wait => {
                if showing {
                    hide_tab(&app);
                    showing = false;
                }
                continue;
            }
            Watch::Look => {}
        }
        let Some(front) = element::foreground() else { continue };
        // One of Clipframes' own windows in front (Settings, History, the tab itself) says
        // nothing about where the user is working, and is never read as if it were a page.
        if front.pid == std::process::id() as i32 {
            continue;
        }
        let key = (front.pid, front.title.clone());
        if seen.as_ref() != Some(&key) {
            if seen.is_some() && read_at.elapsed() < Duration::from_millis(1500) {
                continue; // titles that change all the time do not get a reading each
            }
            // The address is only worth switching an app's page structure on for where the
            // tab could appear on a site in that app. Otherwise every Chromium and Electron
            // app would have it forced on just for coming to the front while Clipframes runs.
            let wake = core.places.lock().unwrap().has_site(&front.app, places::now());
            let url = if element::permitted() { element::page_at(front.frame.x + front.frame.width / 2.0, front.frame.y + front.frame.height / 2.0, wake) } else { String::new() };
            *core.front.lock().unwrap() = Some(Place::new(&front.app, &url));
            seen = Some(key);
            read_at = Instant::now();
        }
        let wanted = core.front.lock().unwrap().as_ref().is_some_and(|p| core.places.lock().unwrap().wants(p, places::now()));
        // The tab goes to the display its window is on, and follows when the window moves.
        let middle = (front.frame.x + front.frame.width / 2.0, front.frame.y + front.frame.height / 2.0);
        if wanted != showing || (wanted && middle != placed) {
            if wanted {
                show_tab(&app, middle);
                placed = middle;
            } else {
                hide_tab(&app);
            }
            showing = wanted;
        }
    }
    hide_tab(&app);
    // Nothing knows what is in front any more: the next bar does not open over a guess.
    *app.state::<Core>().front.lock().unwrap() = None;
}

/// Once a minute: apps that were asked for their page structure and have not been looked at
/// for five minutes get to switch it off again. A thread of its own and not part of the
/// place watcher, which does not run while the tab is switched off: a round asks apps for
/// their structure too, and they must be let go either way. Not done during a round either,
/// because a round is over long before this would ever come up.
#[cfg(target_os = "macos")]
fn tidy() {
    loop {
        thread::sleep(Duration::from_secs(60));
        element::sleep_idle(Duration::from_secs(300));
    }
}

/// The round's folder, made the first time something needs it.
fn round_folder(core: &Core) -> (PathBuf, Stamp) {
    let mut folder = core.folder.lock().unwrap();
    let (path, taken) = folder.get_or_insert_with(|| {
        let taken = Stamp::now();
        (store::new_folder(&store::root(), taken), taken)
    });
    let _ = std::fs::create_dir_all(&*path);
    (path.clone(), *taken)
}

/// Writes the round to its folder.
fn save(core: &Core) {
    let round = core.round.lock().unwrap();
    if round.picks.is_empty() {
        return;
    }
    let (path, taken) = round_folder(core);
    let (path, taken) = (&path, &taken);
    match store::save(&round, path, *taken) {
        Ok(notes) => *core.notes.lock().unwrap() = Some(notes),
        Err(e) => eprintln!("could not save the round to {}: {e}", path.display()),
    }
}

/// A window's place on screen in picker units.
fn window_rect(w: &WebviewWindow) -> Option<Rect> {
    let (p, s) = (w.outer_position().ok()?, w.outer_size().ok()?);
    let k = if cfg!(target_os = "macos") { w.scale_factor().ok()? } else { 1.0 };
    Some(Rect { x: p.x as f64 / k, y: p.y as f64 / k, width: s.width as f64 / k, height: s.height as f64 / k })
}

/// Clicks on the bar and on the open comment box are theirs, not picks.
fn refresh_exempt(app: &AppHandle) {
    let core = app.state::<Core>();
    let mut rects = Vec::new();
    for label in [BAR, NOTE] {
        if let Some(w) = app.get_webview_window(label).filter(|w| w.is_visible().unwrap_or(false)) {
            rects.extend(window_rect(&w));
        }
    }
    if let Some(p) = core.picker.lock().unwrap().as_ref() {
        p.set_exempt(rects);
    };
}

/// Clipframes' own windows are left out of screenshots and recordings, its own included.
/// CLIPFRAMES_CAPTURABLE keeps them in, for recording a demo of Clipframes itself.
fn protected() -> bool {
    std::env::var_os("CLIPFRAMES_CAPTURABLE").is_none()
}

fn small_window<'a>(app: &'a AppHandle, label: &str, size: (f64, f64)) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("Clipframes")
        .inner_size(size.0, size.1)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .accept_first_mouse(true)
        .focused(false)
        .visible(false)
        .content_protected(protected())
        .on_page_load(|window, payload| {
            trace(&format!("{}: page {:?}", window.label(), payload.event()));
        })
}

/// A place for one of Clipframes' windows: its top-left corner in the physical pixels of the
/// display it is on, and that display's scale.
struct Spot {
    at: PhysicalPosition<i32>,
    scale: f64,
}

impl Spot {
    fn put(&self, window: &WebviewWindow) {
        let _ = if cfg!(target_os = "macos") {
            // In points, for the reason given in `open_overlays`: macOS converts pixels with
            // the scale of the display the window is on now, not the one it is going to.
            window.set_position(LogicalPosition::new(self.at.x as f64 / self.scale, self.at.y as f64 / self.scale))
        } else {
            window.set_position(self.at)
        };
    }
}

/// Which display a point is on: the first whose rectangle holds it. A display's left and top
/// edges belong to it, its right and bottom edges to the neighbour.
fn monitor_at(monitors: &[Rect], x: f64, y: f64) -> Option<usize> {
    monitors.iter().position(|m| x >= m.x && x < m.x + m.width && y >= m.y && y < m.y + m.height)
}

/// A display, and what the bar's place on it is remembered under.
struct Display {
    monitor: tauri::Monitor,
    /// The display's own name: its model, and a number when two of them are connected.
    id: String,
    /// Every connected display's name. A place is remembered for the display in the company
    /// it was in, so a desk with two screens and the laptop alone each keep their own.
    arrangement: String,
}

/// A name for each display that no other one has. The system names a display by its model,
/// so two of the same kind are told apart by where they are, left to right and top to bottom.
fn display_ids(displays: &[(String, Rect)]) -> Vec<String> {
    let mut order: Vec<usize> = (0..displays.len()).collect();
    order.sort_by(|&a, &b| displays[a].1.x.total_cmp(&displays[b].1.x).then(displays[a].1.y.total_cmp(&displays[b].1.y)));
    let mut ids = vec![String::new(); displays.len()];
    for (at, &i) in order.iter().enumerate() {
        let name = if displays[i].0.is_empty() { "Display" } else { displays[i].0.as_str() };
        let before = order[..at].iter().filter(|&&j| displays[j].0 == displays[i].0).count();
        ids[i] = if before == 0 { name.to_string() } else { format!("{name} ({})", before + 1) };
    }
    ids
}

/// The connected displays as one name, the same whatever order the system lists them in.
fn arrangement(ids: &[String]) -> String {
    let mut ids = ids.to_vec();
    ids.sort();
    ids.join(" + ")
}

/// Every display there is.
fn all_displays(app: &AppHandle) -> Vec<Display> {
    let monitors = app.available_monitors().unwrap_or_default();
    let named: Vec<(String, Rect)> = monitors.iter().map(|m| (m.name().cloned().unwrap_or_default(), display(m).0)).collect();
    let ids = display_ids(&named);
    let arrangement = arrangement(&ids);
    monitors.into_iter().zip(ids).map(|(monitor, id)| Display { monitor, id, arrangement: arrangement.clone() }).collect()
}

/// The display that shows `near` (a point in picker units), or the main one when there is no
/// point or it is on none.
fn display_near(app: &AppHandle, near: Option<(f64, f64)>) -> Option<Display> {
    let mut all = all_displays(app);
    let frames: Vec<Rect> = all.iter().map(|d| display(&d.monitor).0).collect();
    let main = app.primary_monitor().ok().flatten();
    let found = near.and_then(|(x, y)| monitor_at(&frames, x, y)).or_else(|| all.iter().position(|d| Some(d.monitor.position()) == main.as_ref().map(|m| m.position())));
    match found {
        Some(index) => Some(all.swap_remove(index)),
        // The main display is not among the listed ones: it has no remembered place.
        None => main.map(|monitor| Display { monitor, id: String::new(), arrangement: String::new() }),
    }
}

/// Which display shows the most of `rect`, if any shows it at all.
fn display_showing(rect: &Rect, displays: &[Rect]) -> Option<usize> {
    let shared = |d: &Rect| ((rect.x + rect.width).min(d.x + d.width) - rect.x.max(d.x)).max(0.0) * ((rect.y + rect.height).min(d.y + d.height) - rect.y.max(d.y)).max(0.0);
    displays.iter().map(shared).enumerate().filter(|(_, area)| *area > 0.0).max_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
}

/// How far along its free room a window of `size` with its corner at `at` sits inside
/// `area`: 0 is against the left or top edge, 1 against the right or bottom one. Kept instead
/// of a position, so the place holds when the display's resolution or scale changes, and a
/// window partly off the display is remembered as fully on it.
fn fraction(area: &Rect, size: (f64, f64), at: (f64, f64)) -> (f64, f64) {
    let along = |at: f64, from: f64, room: f64| if room > 0.0 && at.is_finite() { ((at - from) / room).clamp(0.0, 1.0) } else { 0.5 };
    (along(at.0, area.x, area.width - size.0), along(at.1, area.y, area.height - size.1))
}

/// Where that puts the window's corner: always with the whole window inside `area`, or in
/// the middle of it when the window is the larger one.
fn placed(area: &Rect, size: (f64, f64), fraction: (f64, f64)) -> (f64, f64) {
    let at = |fraction: f64, from: f64, room: f64| from + room * if room > 0.0 && fraction.is_finite() { fraction.clamp(0.0, 1.0) } else { 0.5 };
    (at(fraction.0, area.x, area.width - size.0), at(fraction.1, area.y, area.height - size.1))
}

/// The part of a display that the system's own bars leave free, in the display's pixels.
fn work_area(m: &tauri::Monitor) -> Rect {
    let area = m.work_area();
    Rect { x: area.position.x as f64, y: area.position.y as f64, width: area.size.width as f64, height: area.size.height as f64 }
}

/// Where the bar opens on a display when it was never moved there: bottom centre. In the
/// display's pixels, like `bar_rect`.
fn home_rect(m: &tauri::Monitor) -> Rect {
    let (pos, size, scale) = (m.position(), m.size(), m.scale_factor());
    let (w, h) = (BAR_SIZE.0 * scale, BAR_SIZE.1 * scale);
    Rect { x: pos.x as f64 + (size.width as f64 - w) / 2.0, y: pos.y as f64 + size.height as f64 - h - 96.0 * scale, width: w, height: h }
}

/// Where the bar goes on a display: where it was last dragged to there, or else bottom centre.
fn bar_rect(app: &AppHandle, shown: &Display) -> Rect {
    let home = home_rect(&shown.monitor);
    let remembered = app.state::<Core>().settings.lock().unwrap().bar_places.get(&shown.arrangement).and_then(|places| places.get(&shown.id)).copied();
    match remembered {
        Some(fraction) => {
            let (x, y) = placed(&work_area(&shown.monitor), (home.width, home.height), fraction);
            Rect { x, y, ..home }
        }
        None => home,
    }
}

/// Where the bar goes: its place on the display that shows `near`.
fn bar_spot(app: &AppHandle, near: Option<(f64, f64)>) -> Option<Spot> {
    let shown = display_near(app, near)?;
    let bar = bar_rect(app, &shown);
    Some(Spot { at: PhysicalPosition::new(bar.x as i32, bar.y as i32), scale: shown.monitor.scale_factor() })
}

/// The bar moved. When that was the user's hand on the grip, it is followed up once the bar
/// has come to rest (`bar_settle`). Called on the main thread for every step of a drag, so it
/// only leaves a note. Also called for the press on the grip itself (`moved` false).
fn bar_moved(app: &AppHandle, moved: bool) {
    let core = app.state::<Core>();
    if core.gripped.lock().unwrap().is_none() {
        return;
    }
    if moved {
        core.bar_dragged.store(true, Ordering::SeqCst);
    }
    // Without waiting: this is the main thread, and a pause in a drag may have let go.
    if let Ok(picker) = core.picker.try_lock() {
        if let Some(picker) = picker.as_ref() {
            picker.hold(true);
        }
    }
    core.bar_moved.store(true, Ordering::SeqCst);
    if !core.bar_timer.swap(true, Ordering::SeqCst) {
        let app = app.clone();
        thread::spawn(move || {
            let core = app.state::<Core>();
            loop {
                thread::sleep(BAR_SETTLE);
                if !core.bar_moved.swap(false, Ordering::SeqCst) {
                    break;
                }
            }
            core.bar_timer.store(false, Ordering::SeqCst);
            bar_settle(&app);
        });
    }
}

/// The bar has come to rest where the user dragged it. Clicks on it are its own where it is
/// now, and what was under it before can be picked. The place is remembered for the display
/// it is on; the bar itself stays where it was let go, also when part of it is off the screen.
fn bar_settle(app: &AppHandle) {
    let core = app.state::<Core>();
    // Pointing goes on from here, whether or not the button was seen coming up.
    if let Some(picker) = core.picker.lock().unwrap().as_ref() {
        picker.hold(false);
    }
    refresh_exempt(app);
    // A press on the grip that moved nothing leaves the remembered place as it is.
    if !core.bar_dragged.swap(false, Ordering::SeqCst) {
        return;
    }
    let Some(gripped) = core.gripped.lock().unwrap().clone() else { return };
    let Some(rect) = app.get_webview_window(BAR).filter(|w| w.is_visible().unwrap_or(false)).and_then(|w| window_rect(&w)) else { return };
    let all = all_displays(app);
    let frames: Vec<Rect> = all.iter().map(|d| display(&d.monitor).0).collect();
    let Some(shown) = display_showing(&rect, &frames).map(|i| &all[i]) else { return };
    // A display was plugged in or taken away since the press: where the system then put the
    // bar is not a place the user chose.
    if shown.arrangement != gripped || gripped.is_empty() {
        return;
    }
    // Picker units, like the window's own rectangle.
    let unit = if cfg!(target_os = "macos") { shown.monitor.scale_factor() } else { 1.0 };
    let area = work_area(&shown.monitor);
    let area = Rect { x: area.x / unit, y: area.y / unit, width: area.width / unit, height: area.height / unit };
    let now = fraction(&area, (rect.width, rect.height), (rect.x, rect.y));
    {
        let mut settings = core.settings.lock().unwrap();
        let places = settings.bar_places.entry(gripped).or_default();
        // A pause in the middle of a drag, or the same place again: nothing new to keep.
        if places.get(&shown.id).is_some_and(|old| (old.0 - now.0).abs() < 0.002 && (old.1 - now.1).abs() < 0.002) {
            return;
        }
        places.insert(shown.id.clone(), now);
    }
    save_settings(app);
    telemetry::event("bar_moved", json!({ "home": false }));
}

/// The bar, on screen, on the display the pointer is on: that is where the user is working.
/// A kept one is moved and shown; a new one is built where it belongs and already visible,
/// so nothing waits for a second step once the web view is up.
fn show_bar(app: &AppHandle) -> Option<WebviewWindow> {
    // From here until the grip is pressed, the bar only moves because Clipframes puts it.
    let core = app.state::<Core>();
    *core.gripped.lock().unwrap() = None;
    core.bar_dragged.store(false, Ordering::SeqCst);
    let spot = bar_spot(app, element::pointer());
    if let Some(bar) = app.get_webview_window(BAR) {
        if let Some(spot) = &spot {
            spot.put(&bar);
        }
        let _ = bar.show();
        return Some(bar);
    }
    let mut builder = small_window(app, BAR, BAR_SIZE).visible(true);
    if let Some(spot) = &spot {
        builder = builder.position(spot.at.x as f64 / spot.scale, spot.at.y as f64 / spot.scale);
    }
    let bar = builder.build().ok()?;
    // The builder is given the place in the display's own units and finds the display from
    // that. With displays of different scales the same numbers can fit two of them, so the
    // place is said once more in a way that cannot be misread.
    if let Some(spot) = &spot {
        spot.put(&bar);
    }
    Some(bar)
}

/// One click-through window per display. They only paint; the picker owns the input.
fn open_overlays(app: &AppHandle) -> Vec<Screen> {
    let mut screens = Vec::new();
    for (i, m) in app.available_monitors().unwrap_or_default().iter().enumerate() {
        let label = format!("{OVERLAY}{i}");
        // One left over from a round that ended badly is used again: a label can only be
        // built once, and a display without its overlay shows no highlight and no marks.
        let built = app.get_webview_window(&label).map(Ok).unwrap_or_else(|| {
            WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
                .title("Clipframes")
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible_on_all_workspaces(true)
                .focused(false)
                .visible(false)
                .content_protected(protected())
                .build()
        });
        let Ok(w) = built else { continue };
        let (pos, size) = (*m.position(), *m.size());
        let (frame, css) = display(m);
        if cfg!(target_os = "macos") {
            // In points. macOS turns a position or size given in pixels into points with the
            // scale of the display the window is on at that moment, not the one it is going
            // to. With a Retina and an ordinary display side by side that put the overlay in
            // the wrong place at the wrong size, and every highlight with it.
            let _ = w.set_position(LogicalPosition::new(frame.x, frame.y));
            let _ = w.set_size(LogicalSize::new(frame.width, frame.height));
        } else {
            let _ = w.set_position(PhysicalPosition::new(pos.x, pos.y));
            let _ = w.set_size(PhysicalSize::new(size.width, size.height));
        }
        let _ = w.set_ignore_cursor_events(true);
        let _ = w.show();
        screens.push(Screen { label, frame, css });
    }
    screens
}

/// A display in picker units, and how many CSS pixels one unit is there.
fn display(m: &tauri::Monitor) -> (Rect, f64) {
    let (pos, size, scale) = (m.position(), m.size(), m.scale_factor());
    // macOS reports points, everything else physical pixels.
    let unit = if cfg!(target_os = "macos") { scale } else { 1.0 };
    (Rect { x: pos.x as f64 / unit, y: pos.y as f64 / unit, width: size.width as f64 / unit, height: size.height as f64 / unit }, unit / scale)
}

/// Every display in picker units.
fn displays(app: &AppHandle) -> Vec<Rect> {
    let known: Vec<Rect> = app.state::<Core>().screens.lock().unwrap().iter().map(|s| s.frame).collect();
    if !known.is_empty() {
        return known;
    }
    // A pick can come before the overlays are up: input starts first.
    app.available_monitors().unwrap_or_default().iter().map(|m| display(m).0).collect()
}

/// The part of `rect` on the display that shows the most of it, if any display shows it at
/// all. A display left of or above the main one has negative coordinates, and an app may
/// report an element far larger than any screen (the container of a long page): a picture is
/// only ever of what one display shows.
fn on_display(rect: &Rect, displays: &[Rect]) -> Option<Rect> {
    let shared = |d: &Rect| {
        let (x, y) = (rect.x.max(d.x), rect.y.max(d.y));
        let (right, bottom) = ((rect.x + rect.width).min(d.x + d.width), (rect.y + rect.height).min(d.y + d.height));
        (right > x && bottom > y).then(|| Rect { x, y, width: right - x, height: bottom - y })
    };
    displays.iter().filter_map(shared).max_by(|a, b| (a.width * a.height).total_cmp(&(b.width * b.height)))
}

fn close_round_windows(app: &AppHandle) {
    app.state::<Core>().screens.lock().unwrap().clear();
    // Every overlay there is, not only the ones this round knows about.
    let mut going = Vec::new();
    for (label, w) in app.webview_windows() {
        if label == NOTE || label.starts_with(OVERLAY) {
            let _ = w.destroy();
            going.push(label);
        }
    }
    // Destroying is only asked for here; the window is gone once the main thread has done
    // it. An open that came straight after would find the window still listed, use it for
    // the new round, and lose it a moment later. So the close waits until they are gone,
    // which the next open cannot overtake: both hold the same lock.
    let asked = Instant::now();
    while going.iter().any(|label| app.get_webview_window(label).is_some()) && asked.elapsed() < Duration::from_millis(500) {
        thread::sleep(Duration::from_millis(5));
    }
}

fn bar_visible(app: &AppHandle) -> bool {
    app.get_webview_window(BAR).is_some_and(|w| w.is_visible().unwrap_or(false))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Turn {
    Open,
    Close,
    Stay,
}

/// What a request does, given what is running. `toggle` is the shortcut; anything else (the
/// tray, the tab, a second launch) asks to open.
fn turn(toggle: bool, picking: bool, bar_visible: bool) -> Turn {
    match (toggle, picking, bar_visible) {
        (true, false, false) => Turn::Open,
        // The shortcut always gets the user out of a round, also one whose bar is not there.
        (true, _, _) => Turn::Close,
        (false, true, true) => Turn::Stay,
        // Clicks are being taken and there is nothing to see or press: end it.
        (false, true, false) => Turn::Close,
        (false, false, _) => Turn::Open,
    }
}

fn take_turn(app: &AppHandle, toggle: bool) {
    let core = app.state::<Core>();
    let _gate = core.gate.lock().unwrap();
    let picking = core.picker.lock().unwrap().is_some();
    match turn(toggle, picking, bar_visible(app)) {
        Turn::Open => open_now(app),
        Turn::Close => close_now(app),
        Turn::Stay => {}
    }
}

/// Opens the bar and starts picking. Call from a thread that may wait: it builds windows.
fn open(app: &AppHandle) {
    take_turn(app, false);
}

fn open_now(app: &AppHandle) {
    let core = app.state::<Core>();
    trace("open: begin");
    let started = Instant::now();
    let via = telemetry::take_via();
    core.turn.fetch_add(1, Ordering::SeqCst);
    *core.opened.lock().unwrap() = Some(started);
    *core.began.lock().unwrap() = Some(started);
    *core.round.lock().unwrap() = Round::default();
    *core.noting.lock().unwrap() = None;
    // The round gets a new comment box, which numbers its messages from one again.
    core.note_seq.store(0, Ordering::SeqCst);
    *core.shown.lock().unwrap() = None;
    *core.tool.lock().unwrap() = Kind::Element;
    let front = core.front.lock().unwrap().clone();
    *core.over.lock().unwrap() = front;
    hide_tab(app);
    core.files.store(0, Ordering::SeqCst);
    *core.folder.lock().unwrap() = None;
    *core.notes.lock().unwrap() = None;
    // Pointing at elements needs only this one. Pictures need another permission on macOS,
    // which is asked about when a picture tool is chosen (`tool_set`), so someone who only
    // wants elements, or has said no to the other, can still work.
    *core.trouble.lock().unwrap() = (!element::permitted()).then(|| "permission".to_string());

    let was_warm = app.get_webview_window(BAR).is_some();
    if was_warm {
        // A kept bar is shown at once, long before the rest is up: it must not come back
        // showing the count of the round before.
        let _ = app.emit("round", view(app));
    }
    if core.trouble.lock().unwrap().is_none() {
        // Input first: clicks are picks from here on. The windows that show it follow, and
        // starting a web view is the slow part of opening.
        let handle = app.clone();
        match Picker::start(move |event| on_event(&handle, event)) {
            Ok(picker) => *core.picker.lock().unwrap() = Some(picker),
            Err(message) => *core.trouble.lock().unwrap() = Some(message),
        }
    }
    let picking = started.elapsed();
    trace("open: input started");
    shot::warm();

    if show_bar(app).is_none() {
        // No bar means no way to see or end the round.
        drop(core.picker.lock().unwrap().take());
        return;
    }
    trace("open: bar on screen");
    if let Some(trouble) = core.trouble.lock().unwrap().as_deref() {
        // "permission" is a known state; anything else is the picker's own error message.
        let kind = if trouble == "permission" { "permission" } else { "picker" };
        telemetry::event("round_blocked", json!({ "via": via, "why": kind }));
        if kind == "picker" {
            telemetry::error("picker", trouble, "picker::start");
        }
    }
    if core.trouble.lock().unwrap().is_some() {
        let _ = app.emit("round", view(app));
        return;
    }
    refresh_exempt(app);
    let screens = open_overlays(app);
    trace("open: overlays built");
    *core.screens.lock().unwrap() = screens;
    let _ = small_window(app, NOTE, (NOTE_SIZE.0, if note_style() == NoteStyle::Line { LINE_HEIGHT.0 } else { NOTE_SIZE.1 })).build();
    trace("open: comment box built");
    // The overlays were built after the bar, so they sit above it: put the bar back on top,
    // or a pick's outline would be drawn across it.
    if let Some(bar) = app.get_webview_window(BAR) {
        let _ = bar.set_always_on_top(false);
        let _ = bar.set_always_on_top(true);
    }
    refresh_exempt(app);
    publish(app);
    eprintln!("open ({}): picking after {:.0} ms, all windows after {:.0} ms", if was_warm { "warm" } else { "cold" }, picking.as_secs_f64() * 1000.0, started.elapsed().as_secs_f64() * 1000.0);
    telemetry::event("round_opened", json!({ "via": via, "warm": was_warm, "ms_to_input": picking.as_millis() as u64, "ms_to_windows": started.elapsed().as_millis() as u64, "displays": core.screens.lock().unwrap().len() }));
}

/// Ends the round and hides everything. What was picked stays on the clipboard. Call from a
/// thread that may wait: stopping the picker waits for its threads.
fn close(app: &AppHandle) {
    let core = app.state::<Core>();
    let _gate = core.gate.lock().unwrap();
    close_now(app);
}

fn close_now(app: &AppHandle) {
    trace("close: begin");
    let core = app.state::<Core>();
    // The overlays are there for as long as a round is, also one that has stopped taking
    // input because the screen could not be captured.
    let was_open = core.picker.lock().unwrap().is_some() || !core.screens.lock().unwrap().is_empty();
    // A clip still recording is finished first, so it is kept.
    stop_recording(app);
    let waiting = Instant::now();
    while core.recording.lock().unwrap().is_some() && waiting.elapsed() < Duration::from_secs(3) {
        thread::sleep(Duration::from_millis(30));
    }
    // Taken out first and dropped unlocked: stopping the picker waits for its threads.
    let picker = core.picker.lock().unwrap().take();
    trace("close: picker taken");
    drop(picker);
    trace("close: picker stopped");
    *core.noting.lock().unwrap() = None;
    // What was being typed in the comment box goes with the round, however it was closed.
    settle_note(app);
    if was_open {
        let round = core.round.lock().unwrap();
        let count = |kind: Kind| round.picks.iter().filter(|p| p.kind == kind).count();
        let seconds = core.began.lock().unwrap().take().map(|t| t.elapsed().as_secs()).unwrap_or(0);
        telemetry::event("round_closed", json!({ "picks": round.picks.len(), "elements": count(Kind::Element), "areas": count(Kind::Area), "clips": count(Kind::Clip), "notes": round.picks.iter().filter(|p| !p.note.is_empty()).count(), "seconds": seconds }));
    }
    close_round_windows(app);
    trace("close: windows closed");
    // A bar let go a moment ago may not have had its place remembered yet.
    if core.bar_moved.swap(false, Ordering::SeqCst) {
        bar_settle(app);
    }
    *core.gripped.lock().unwrap() = None;
    if let Some(bar) = app.get_webview_window(BAR) {
        let _ = bar.hide();
    }
    let _ = app.emit("round", view(app));

    // Let the bar go too, unless it is opened again first.
    let turn = core.turn.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(KEEP_WARM);
        let core = app.state::<Core>();
        // Held so that an open cannot begin between the check and the bar going away.
        let _gate = core.gate.lock().unwrap();
        if core.turn.load(Ordering::SeqCst) == turn && core.picker.lock().unwrap().is_none() {
            if let Some(bar) = app.get_webview_window(BAR) {
                let _ = bar.destroy();
            }
        }
    });
}

fn toggle(app: &AppHandle) {
    take_turn(app, true);
}

/// Runs `f` off the calling thread. The picker's own threads must never stop the picker, and
/// the main thread must never wait for a window it has to build itself.
fn later(app: &AppHandle, f: fn(&AppHandle)) {
    let app = app.clone();
    thread::spawn(move || f(&app));
}

fn on_event(app: &AppHandle, event: Event) {
    let core = app.state::<Core>();
    match event {
        Event::Hover { x, y, element, .. } => {
            if note_style() == NoteStyle::Yield {
                yield_note(app, x, y);
            }
            let frame = element.as_ref().map(|e| e.frame).filter(|f| f.width > 0.0 && f.height > 0.0);
            {
                let mut shown = core.shown.lock().unwrap();
                if *shown == frame {
                    return;
                }
                *shown = frame;
            }
            let label = element.as_ref().map(ElementInfo::headline).unwrap_or_default();
            for screen in core.screens.lock().unwrap().iter() {
                let _ = app.emit_to(screen.label.as_str(), "hover", HoverView { rect: frame.map(|f| screen.local(&f)), label: label.clone() });
            }
        }
        Event::Pick { element, token, read_ms, .. } => {
            // With CLIPFRAMES_TRACE these lines say where the time from click to comment box
            // goes: each is stamped, and says how long after the pick arrived it was.
            let arrived = Instant::now();
            let after = |what: &str| trace(&format!("pick {token}: {what} (+{:.0} ms)", arrived.elapsed().as_secs_f64() * 1000.0));
            trace(&format!("pick {token}: element read, {read_ms:.0} ms after the click"));
            let frame = element.frame;
            // A picture of the element with a little space around it. Not having one is fine.
            // Taken before anything of Clipframes' own appears near the element.
            let pad = if cfg!(windows) { 18.0 } else { 12.0 };
            let around = Rect { x: frame.x - pad, y: frame.y - pad, width: frame.width + pad * 2.0, height: frame.height + pad * 2.0 };
            // Where the system says Clipframes may not take pictures (macOS without Screen
            // Recording) none is tried: trying is what makes the system ask, and its question
            // would come up while every click is being taken as a pick.
            let (image, pixels) = on_display(&around, &displays(app)).filter(|_| shot::permitted()).and_then(|seen| snap(&core, &seen, ELEMENT_WIDTH)).unwrap_or_default();
            after("picture done");
            let report = json!({ "kind": "element", "picture": !image.is_empty(), "selector": !element.selector().is_empty(), "named": !element.name.is_empty(), "web": !element.url.is_empty() });
            let place = element.clone();
            let index = core.round.lock().unwrap().push(Pick { id: token, element, image, pixels, ..Default::default() });
            *core.noting.lock().unwrap() = Some(index);
            after("pick added");
            // The comment box first: it is what the user is waiting for. What touches the
            // disk and the clipboard comes after it.
            show_note(app, &frame);
            after("comment box shown");
            remember(app, &place);
            telemetry::event("pick_added", report);
            publish(app);
            after("published");
        }
        // Which one it is and under what heading, found after the pick was shown.
        Event::Located { token, occurrence, heading, took_ms } => {
            trace(&format!("pick {token}: which one read in {took_ms:.0} ms: {occurrence:?}, heading {}", heading.is_some()));
            if occurrence.is_none() && heading.is_none() {
                return;
            }
            // By what the pick is known by, not where it is: picks removed since have moved
            // it, and it may be gone, or its round over and another begun.
            if !core.round.lock().unwrap().locate(token, occurrence, heading) {
                return trace(&format!("pick {token}: no longer there"));
            }
            let open = core.picker.lock().unwrap().is_some();
            match late(open, !open && clipboard_is_ours(app)) {
                Late::Publish => publish(app),
                Late::SaveAndCopy => {
                    save(&core);
                    let reference = view(app).reference;
                    copy_text(app, &reference);
                }
                Late::Save => save(&core),
            }
            trace(&format!("pick {token}: which one published"));
        }
        Event::Drag { rect } => show_area(app, rect, false),
        // These arrive on the input thread, which must not wait for a screenshot.
        Event::Area { rect } => {
            let app = app.clone();
            thread::spawn(move || if *app.state::<Core>().tool.lock().unwrap() == Kind::Clip { start_recording(&app, rect) } else { add_area(&app, rect) });
        }
        Event::Click { element, .. } => {
            if let Some(recording) = core.recording.lock().unwrap().as_ref() {
                recording.clicks.lock().unwrap().push((recording.started.elapsed().as_secs_f64(), element.map(|e| e.headline()).unwrap_or_default()));
            }
        }
        Event::Cancel => {
            trace("esc");
            later(app, escape)
        }
    }
}

/// Takes a picture of part of the screen into the round's folder. Returns its name and size.
fn snap(core: &Core, rect: &Rect, widest: u32) -> Option<(String, (u32, u32))> {
    let (folder, _) = round_folder(core);
    let name = format!("{}.png", core.files.fetch_add(1, Ordering::SeqCst) + 1);
    match shot::capture_to_file(rect, &folder.join(&name), Some(widest)) {
        Ok(pixels) => Some((name, pixels)),
        Err(e) => {
            trace(&format!("no picture: {e}"));
            telemetry::error("capture", &e.to_string(), "app::snap");
            None
        }
    }
}

/// Where an area was taken: the app, window and page under its middle, with the area as frame.
fn place_of(rect: &Rect) -> ElementInfo {
    let under = element::element_full_at(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0).unwrap_or_default();
    ElementInfo { app: under.app, pid: under.pid, window: under.window, url: under.url, frame: *rect, ..Default::default() }
}

/// Draws (or clears) the area being dragged or recorded on every overlay.
fn show_area(app: &AppHandle, rect: Option<Rect>, recording: bool) {
    for screen in app.state::<Core>().screens.lock().unwrap().iter() {
        let _ = app.emit_to(screen.label.as_str(), "area", AreaView { rect: rect.map(|r| screen.local(&r)), recording });
    }
}

/// The screen could not be captured: say so in the bar instead of adding an empty pick.
fn cannot_capture(app: &AppHandle) {
    telemetry::error("capture", "the screen could not be captured", "app::cannot_capture");
    no_pictures(app);
}

/// The picture tools cannot be used: the bar says so, with a way to the system's settings
/// and a way back to pointing at elements, which needs no pictures.
fn no_pictures(app: &AppHandle) {
    let core = app.state::<Core>();
    // "screen-asked": the user has been to the settings in this run. macOS often goes on
    // saying no until the app is started again, and the bar says that instead.
    let state = if !cfg!(target_os = "macos") {
        "The screen could not be captured."
    } else if core.asked_screen.load(Ordering::SeqCst) {
        "screen-asked"
    } else {
        "screen"
    };
    *core.trouble.lock().unwrap() = Some(state.into());
    // Until a tool is chosen again every click goes to the app it is on. Left as it was, the
    // round would swallow the clicks the user now needs elsewhere: on the system's own
    // question about the permission, or in System Settings. Esc still ends the round.
    if let Some(picker) = core.picker.lock().unwrap().as_ref() {
        picker.set_mode(Mode::Watch);
    }
    // The round's folder was made for the picture that failed. With nothing picked it is
    // empty, and would sit among the captures for good.
    {
        let round = core.round.lock().unwrap();
        if round.picks.is_empty() {
            if let Some((path, _)) = core.folder.lock().unwrap().take() {
                let _ = std::fs::remove_dir_all(path);
            }
            core.files.store(0, Ordering::SeqCst);
        }
    }
    let _ = app.emit("round", view(app));
}

fn add_area(app: &AppHandle, rect: Rect) {
    let core = app.state::<Core>();
    show_area(app, None, false);
    // One frame for the outline to leave the screen, where it is not protected from capture.
    if !protected() {
        thread::sleep(Duration::from_millis(60));
    }
    let Some((image, pixels)) = snap(&core, &rect, AREA_WIDTH) else { return cannot_capture(app) };
    telemetry::event("pick_added", json!({ "kind": "area", "picture": true, "width": pixels.0, "height": pixels.1 }));
    let element = place_of(&rect);
    remember(app, &element);
    let index = core.round.lock().unwrap().push(Pick { kind: Kind::Area, element, image, pixels, ..Default::default() });
    *core.noting.lock().unwrap() = Some(index);
    show_note(app, &rect);
    publish(app);
}

fn start_recording(app: &AppHandle, rect: Rect) {
    let core = app.state::<Core>();
    let (folder, _) = round_folder(&core);
    let name = (core.files.fetch_add(1, Ordering::SeqCst) + 1).to_string();
    let dir = folder.join(&name);
    if std::fs::create_dir_all(&dir).is_err() {
        return cannot_capture(app);
    }
    let recording = Recording { started: Instant::now(), stop: Arc::new(AtomicBool::new(false)), clicks: Arc::new(Mutex::new(Vec::new())) };
    *core.recording.lock().unwrap() = Some(recording.clone());
    if let Some(picker) = core.picker.lock().unwrap().as_ref() {
        picker.set_mode(Mode::Watch);
    }
    if core.noting.lock().unwrap().take().is_some() {
        if let Some(note) = app.get_webview_window(NOTE) {
            let _ = note.hide();
        }
    }
    refresh_exempt(app);
    show_area(app, Some(rect), true);
    let _ = app.emit("round", view(app));

    let app = app.clone();
    thread::spawn(move || {
        let (mut frames, mut pixels, mut times) = (0u32, (0u32, 0u32), Vec::new());
        let (mut tries, mut failed) = (0u32, 0u32);
        loop {
            let at = recording.started.elapsed();
            if recording.stop.load(Ordering::SeqCst) || at > LONGEST_CLIP {
                break;
            }
            tries += 1;
            match shot::capture_to_file(&rect, &dir.join(format!("{:03}.png", frames + 1)), Some(FRAME_WIDTH)) {
                Ok(size) => {
                    frames += 1;
                    failed = 0;
                    pixels = size;
                    times.push(at.as_secs_f64());
                }
                Err(_) if frames == 0 => break,
                // A capture that keeps failing (a full disk) ends the clip with what it has.
                Err(_) => {
                    failed += 1;
                    if failed >= 8 {
                        break;
                    }
                }
            }
            if frames % 4 == 0 {
                let _ = app.emit("round", view(&app)); // the clock in the bar
            }
            // By tries, not frames: a failed frame waits its turn like any other, instead of
            // being tried again a hundred times a second.
            let next = FRAME_EVERY * tries;
            thread::sleep(next.saturating_sub(recording.started.elapsed()).max(Duration::from_millis(10)));
        }
        let seconds = recording.started.elapsed().as_secs_f64();
        let core = app.state::<Core>();
        *core.recording.lock().unwrap() = None;
        if let Some(picker) = core.picker.lock().unwrap().as_ref() {
            picker.set_mode(Mode::Area);
        }
        show_area(&app, None, false);
        if frames == 0 {
            let _ = std::fs::remove_dir_all(&dir);
            return cannot_capture(&app);
        }
        // Each click is tied to the first frame taken after it.
        let clicks = recording.clicks.lock().unwrap().iter().map(|(at, what)| Click { at: *at, frame: (times.iter().position(|t| t >= at).unwrap_or(times.len() - 1) + 1) as u32, what: what.clone() }).collect();
        telemetry::event("pick_added", json!({ "kind": "clip", "picture": true, "seconds": seconds.round() as u64, "frames": frames, "clicks": recording.clicks.lock().unwrap().len() }));
        let element = place_of(&rect);
        remember(&app, &element);
        let index = core.round.lock().unwrap().push(Pick { kind: Kind::Clip, element, image: name, pixels, frames, seconds, clicks, ..Default::default() });
        *core.noting.lock().unwrap() = Some(index);
        show_note(&app, &rect);
        publish(&app);
    });
}

/// True if a clip was being recorded.
fn stop_recording(app: &AppHandle) -> bool {
    match app.state::<Core>().recording.lock().unwrap().as_ref() {
        Some(recording) => {
            recording.stop.store(true, Ordering::SeqCst);
            true
        }
        None => false,
    }
}

/// Opens the comment box for a pick: under it, inside its display, or where the style being
/// tried puts it (`note_spot`). It takes the keyboard.
fn show_note(app: &AppHandle, frame: &Rect) {
    let height = if note_style() == NoteStyle::Line { LINE_HEIGHT.0 } else { NOTE_SIZE.1 };
    *app.state::<Core>().noted.lock().unwrap() = Some(Noted { pick: *frame, at: Rect::default(), height, farthest: 0.0, stepped: false });
    place_note(app, true);
}

/// Puts the comment box where it belongs for the pick it is open for.
fn place_note(app: &AppHandle, focus: bool) {
    let core = app.state::<Core>();
    let Some(note) = app.get_webview_window(NOTE) else { return };
    let Some(noted) = core.noted.lock().unwrap().clone() else { return };
    let frame = &noted.pick;
    let screens = core.screens.lock().unwrap();
    let Some(screen) = screens.iter().find(|s| s.contains(frame.x + frame.width / 2.0, frame.y + frame.height / 2.0)).or(screens.first()) else { return };
    let (w, h, gap) = (NOTE_SIZE.0 / screen.css, noted.height / screen.css, 8.0 / screen.css);
    let (x, y) = note_spot(note_style(), &screen.frame, frame, (w, h), gap, noted.stepped);
    drop(screens);
    if let Some(noted) = core.noted.lock().unwrap().as_mut() {
        noted.at = Rect { x, y, width: w, height: h };
    }
    if note_style() == NoteStyle::Line {
        let _ = note.set_size(LogicalSize::new(NOTE_SIZE.0, noted.height));
    }
    let _ = if cfg!(target_os = "macos") {
        note.set_position(LogicalPosition::new(x, y))
    } else {
        note.set_position(PhysicalPosition::new(x as i32, y as i32))
    };
    if focus {
        let _ = note.show();
        let _ = note.set_focus();
    }
    refresh_exempt(app);
}

/// The yielding comment box: the pointer is at this point, outside the box. When it has come
/// for what the box covers, the box steps to another side of its pick, once.
fn yield_note(app: &AppHandle, x: f64, y: f64) {
    let core = app.state::<Core>();
    if core.noting.lock().unwrap().is_none() {
        return;
    }
    {
        let mut noted = core.noted.lock().unwrap();
        let Some(noted) = noted.as_mut().filter(|n| !n.stepped && n.at.width > 0.0) else { return };
        let now = away(&noted.at, x, y);
        noted.farthest = noted.farthest.max(now);
        if !comes_for(noted.farthest, now) {
            return;
        }
        noted.stepped = true;
    }
    place_note(app, false);
}

/// Esc stops a recording, or closes the comment box if one is open, or else ends the round.
fn escape(app: &AppHandle) {
    if stop_recording(app) {
        return;
    }
    if !hide_note(app) {
        close(app);
    }
}

/// True if a comment box was open.
fn hide_note(app: &AppHandle) -> bool {
    let core = app.state::<Core>();
    let was_open = core.noting.lock().unwrap().take().is_some();
    trace(if was_open { "note: closing" } else { "note: none open" });
    if was_open {
        if let Some(note) = app.get_webview_window(NOTE) {
            let _ = note.hide();
        }
        refresh_exempt(app);
        publish(app);
        trace("note: closed");
    }
    was_open
}

#[tauri::command]
fn round_state(app: AppHandle, window: WebviewWindow) -> RoundView {
    if window.label() == BAR {
        if let Some(opened) = app.state::<Core>().opened.lock().unwrap().take() {
            eprintln!("open: bar drawn after {:.0} ms", opened.elapsed().as_secs_f64() * 1000.0);
            trace("open: bar script asked for state");
        }
    }
    view(&app)
}

/// The comment box sends its text as it is typed, so closing the round in any way keeps it.
/// The round has the text at once; the clipboard and the files follow a moment later, once
/// for a run of keystrokes.
///
/// Every keystroke is a message of its own, and nothing promises they arrive in the order
/// they were sent. Each carries a number (`seq`) that the round's comment box counts up from
/// one; a message with a number not above the newest seen is an older text arriving late and
/// is dropped. The newest seen starts at nothing with every round (`open_now`).
#[tauri::command]
fn note_set(app: AppHandle, index: usize, note: String, seq: Option<u64>) {
    let core = app.state::<Core>();
    if seq.is_some_and(|seq| !newer(&core.note_seq, seq)) {
        return;
    }
    core.round.lock().unwrap().set_note(index, &note);
    core.note_unsaved.store(true, Ordering::SeqCst);
    if !core.note_timer.swap(true, Ordering::SeqCst) {
        let app = app.clone();
        thread::spawn(move || {
            thread::sleep(NOTE_SETTLE);
            app.state::<Core>().note_timer.store(false, Ordering::SeqCst);
            settle_note(&app);
        });
    }
}

/// Takes `seq` as the newest number seen if it is, and says whether it was.
fn newer(newest: &AtomicU64, seq: u64) -> bool {
    newest.fetch_max(seq, Ordering::SeqCst) < seq
}

/// Writes out a comment that was typed but not yet saved or copied.
fn settle_note(app: &AppHandle) {
    if app.state::<Core>().note_unsaved.load(Ordering::SeqCst) {
        publish(app);
    }
}

#[tauri::command]
fn note_close(app: AppHandle) {
    hide_note(&app);
}

/// The one-line comment box needs this much height for what is typed in it.
#[tauri::command]
fn note_resize(app: AppHandle, height: f64) {
    if note_style() != NoteStyle::Line || !height.is_finite() {
        return;
    }
    let height = height.clamp(LINE_HEIGHT.0, LINE_HEIGHT.1);
    {
        let core = app.state::<Core>();
        let mut noted = core.noted.lock().unwrap();
        let Some(noted) = noted.as_mut().filter(|n| n.height != height) else { return };
        noted.height = height;
    }
    place_note(&app, false);
}

#[tauri::command]
fn pick_remove(app: AppHandle, index: usize) {
    let core = app.state::<Core>();
    telemetry::event("pick_removed", json!({}));
    let nothing_left = {
        let mut round = core.round.lock().unwrap();
        round.remove(index);
        round.picks.is_empty()
    };
    // The clipboard and the round's folder still hold the pick that was just taken back.
    if nothing_left {
        uncopy(&app);
        let folder = core.folder.lock().unwrap().take();
        if let Some((path, _)) = folder {
            let _ = std::fs::remove_dir_all(path);
        }
        core.files.store(0, Ordering::SeqCst);
        *core.notes.lock().unwrap() = None;
    }
    // Remove is pressed in the comment box, which goes with its pick.
    if !hide_note(&app) {
        publish(&app);
    }
}

#[tauri::command]
fn escape_key(app: AppHandle) {
    trace("esc from a window");
    later(&app, escape);
}

#[tauri::command]
fn round_done(app: AppHandle) {
    later(&app, close);
}

/// Quits: a comment still being typed is written out and waiting reports are sent first.
fn quit(app: &AppHandle) {
    settle_note(app);
    telemetry::flush();
    app.exit(0)
}

/// The bar's "Quit Clipframes", shown when a permission only takes effect in a new start.
#[tauri::command]
fn app_quit(app: AppHandle) {
    quit(&app);
}

/// Opens the system's page for a permission Clipframes needs.
#[tauri::command]
fn permission_open(app: AppHandle, kind: String) {
    use tauri_plugin_opener::OpenerExt;
    // The system's own question first: being asked is what puts Clipframes in the list the
    // user is about to look at.
    let screen = kind.starts_with("screen");
    if screen {
        shot::ask_permission();
        app.state::<Core>().asked_screen.store(true, Ordering::SeqCst);
    } else {
        element::ask_permission();
    }
    let pane = if screen { "Privacy_ScreenCapture" } else { "Privacy_Accessibility" };
    let _ = app.opener().open_url(format!("x-apple.systempreferences:com.apple.preference.security?{pane}"), None::<&str>);
    *app.state::<Core>().trouble.lock().unwrap() = None;
    // The round ends and the bar goes: nothing of Clipframes may be in the way, or taking
    // clicks, while the user is in System Settings.
    later(&app, close);
}

#[tauri::command]
fn tool_set(app: AppHandle, tool: Kind) {
    let core = app.state::<Core>();
    if core.recording.lock().unwrap().is_some() {
        return;
    }
    *core.tool.lock().unwrap() = tool;
    *core.trouble.lock().unwrap() = None;
    // A picture tool where the system says no pictures: until the trial below has shown that
    // one can be taken after all, clicks go to the app they are on, so the system's question,
    // should the trial bring it up, can be answered.
    let on_trial = tool != Kind::Element && !shot::permitted();
    if let Some(picker) = core.picker.lock().unwrap().as_ref() {
        picker.set_mode(if tool == Kind::Element { Mode::Element } else if on_trial { Mode::Watch } else { Mode::Area });
    }
    // The element highlight belongs to the element tool.
    *core.shown.lock().unwrap() = None;
    for screen in core.screens.lock().unwrap().iter() {
        let _ = app.emit_to(screen.label.as_str(), "hover", HoverView { rect: None, label: String::new() });
    }
    let _ = app.emit("round", view(&app));
    // A picture tool where the system says Clipframes may not take pictures (macOS without
    // Screen Recording). The system's answer is known to stay "no" after the user has said
    // yes, until the app is started again, so it is not taken at its word: a small picture
    // is tried, and only if that fails too does the bar say what is missing.
    if on_trial {
        thread::spawn(move || {
            let core = app.state::<Core>();
            let still = || *core.tool.lock().unwrap() == tool && core.trouble.lock().unwrap().is_none();
            let works = shot::works();
            if !still() {
                return;
            }
            if !works {
                return no_pictures(&app);
            }
            if let Some(picker) = core.picker.lock().unwrap().as_ref() {
                picker.set_mode(Mode::Area);
            };
        });
    }
}

#[tauri::command]
fn recording_stop(app: AppHandle) {
    stop_recording(&app);
}

#[tauri::command]
fn history_open(app: AppHandle) {
    later(&app, |app| {
        close(app);
        if let Some(window) = app.get_webview_window(HISTORY) {
            let _ = window.show();
            let _ = window.set_focus();
            return;
        }
        let built = WebviewWindowBuilder::new(app, HISTORY, WebviewUrl::App("index.html".into())).title("Clipframes History").inner_size(680.0, 620.0).min_inner_size(520.0, 320.0).center().background_color(tauri::window::Color(18, 18, 19, 255)).build();
        if let Ok(window) = built {
            let _ = window.set_focus();
        }
    });
}

#[tauri::command]
fn history_list(from: usize, count: usize) -> store::Page {
    store::list(&store::root(), from, count.min(200))
}

/// Puts a past round back on the clipboard.
#[tauri::command]
fn history_copy(app: AppHandle, id: String) -> Result<(), String> {
    let folder = store::folder_of(&store::root(), &id).ok_or("That capture is gone.")?;
    let round = store::load(&folder).ok_or("That capture could not be read.")?;
    let text = round.reference(folder.join("notes.md").to_str());
    copy_text(&app, &text);
    telemetry::event("history_copied", json!({ "picks": round.picks.len() }));
    Ok(())
}

#[tauri::command]
fn history_reveal(app: AppHandle, id: String) {
    use tauri_plugin_opener::OpenerExt;
    if let Some(folder) = store::folder_of(&store::root(), &id) {
        let _ = app.opener().open_path(folder.display().to_string(), None::<&str>);
    }
}

#[tauri::command]
fn history_delete(id: String) -> Result<(), String> {
    let folder = store::folder_of(&store::root(), &id).ok_or("That capture is gone.")?;
    telemetry::event("history_deleted", json!({}));
    std::fs::remove_dir_all(folder).map_err(|e| e.to_string())
}

/// Turns the tab on or off for the place the bar is open over.
#[tauri::command]
fn place_auto_set(app: AppHandle, on: bool) {
    let core = app.state::<Core>();
    if let Some(place) = core.over.lock().unwrap().as_ref() {
        let mut places = core.places.lock().unwrap();
        places.set_auto(place, on, places::now());
        telemetry::event("place_auto_set", json!({ "on": on }));
        if let Some(dir) = config_dir(&app) {
            let _ = places.save(&dir);
        }
    }
    let _ = app.emit("round", view(&app));
}

/// The tab was clicked.
#[tauri::command]
fn tab_open(app: AppHandle) {
    telemetry::via("tab");
    later(&app, open);
}

/// The bar's grip was pressed: the system moves the bar with the pointer until the button
/// comes up. The press went to the bar, so the picker takes nothing until then.
#[tauri::command]
fn bar_grip(app: AppHandle, window: WebviewWindow) {
    let arrangement = all_displays(&app).first().map(|d| d.arrangement.clone()).unwrap_or_default();
    *app.state::<Core>().gripped.lock().unwrap() = Some(arrangement);
    // As for any move: the picker holds off, and lets go again once the bar rests, also
    // when the grip was only pressed and the bar never moved.
    bar_moved(&app, false);
    let _ = window.start_dragging();
}

/// The grip was double-clicked: the bar goes back to bottom centre of the display it is on,
/// and opens there from now on.
#[tauri::command]
fn bar_home(app: AppHandle, window: WebviewWindow) {
    let core = app.state::<Core>();
    // The first of the two clicks was a press on the grip. What follows is not a drag.
    *core.gripped.lock().unwrap() = None;
    core.bar_moved.store(false, Ordering::SeqCst);
    core.bar_dragged.store(false, Ordering::SeqCst);
    let Some(rect) = window_rect(&window) else { return };
    let all = all_displays(&app);
    let frames: Vec<Rect> = all.iter().map(|d| display(&d.monitor).0).collect();
    let Some(index) = display_showing(&rect, &frames) else { return };
    let forgotten = core.settings.lock().unwrap().bar_places.get_mut(&all[index].arrangement).is_some_and(|places| places.remove(&all[index].id).is_some());
    if forgotten {
        save_settings(&app);
        telemetry::event("bar_moved", json!({ "home": true }));
    }
    let middle = (frames[index].x + frames[index].width / 2.0, frames[index].y + frames[index].height / 2.0);
    if let Some(spot) = bar_spot(&app, Some(middle)) {
        spot.put(&window);
    }
    refresh_exempt(&app);
}

/// What the settings window draws.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsView {
    shortcut: String,
    shortcut_label: String,
    shortcut_works: bool,
    launch_at_login: bool,
    share_usage: bool,
    /// False in a build that has nowhere to send usage to: the switch is not shown.
    usage_available: bool,
    version: String,
    update: String,
    mac: bool,
    /// Whether Claude Code's own settings let it read captures without asking (claude.rs).
    claude_read: claude::State,
    /// What that takes: shown for adding by hand when Clipframes will not change the file.
    claude_rules: Vec<String>,
    /// Whether the tab appears at all.
    show_tab: bool,
    /// The apps and sites it appears in now.
    tab_places: Vec<ShownPlace>,
}

#[derive(Debug, Clone, Serialize)]
struct ShownPlace {
    app: String,
    host: String,
    /// "Google Chrome · localhost:3000", "Slack".
    name: String,
}

/// Claude Code's settings file, and the rules in it that cover the captures folder.
fn claude_read(app: &AppHandle) -> Option<(PathBuf, Vec<String>)> {
    let home = app.path().home_dir().ok()?;
    Some((claude::settings_file(&home), claude::rules(&store::root(), &home)))
}

fn settings_view(app: &AppHandle) -> SettingsView {
    let core = app.state::<Core>();
    let settings = core.settings.lock().unwrap().clone();
    let shortcut_works = *core.shortcut_works.lock().unwrap();
    let update = core.update.lock().unwrap().clone();
    // Read from Claude Code's file every time: it is the file that decides, and it can change
    // while this window is open.
    let (claude_read, claude_rules) = claude_read(app).map(|(file, rules)| (claude::state(&file, &rules), rules)).unwrap_or((claude::State::Missing, Vec::new()));
    let tab_places = core.places.lock().unwrap().shown(places::now()).into_iter().map(|p| ShownPlace { name: p.name(), app: p.app, host: p.host }).collect();
    SettingsView {
        shortcut_label: settings::label(&settings.shortcut, cfg!(target_os = "macos")),
        shortcut: settings.shortcut,
        shortcut_works,
        launch_at_login: settings.launch_at_login,
        share_usage: settings.share_usage,
        usage_available: telemetry::available(),
        version: app.package_info().version.to_string(),
        update,
        mac: cfg!(target_os = "macos"),
        claude_read,
        claude_rules,
        show_tab: settings.show_tab,
        tab_places,
    }
}

fn save_settings(app: &AppHandle) {
    let settings = app.state::<Core>().settings.lock().unwrap().clone();
    if let Ok(dir) = app.path().app_config_dir() {
        if let Err(e) = settings::save(&dir, &settings) {
            eprintln!("could not save settings: {e}");
        }
    }
}

/// Takes the shortcut. Fails when it is malformed or another app owns it.
fn bind_shortcut(app: &AppHandle, shortcut: &str) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                telemetry::via("shortcut");
                later(app, toggle);
            }
        })
        .map_err(|e| e.to_string())
}

fn refresh_menu(app: &AppHandle) {
    let core = app.state::<Core>();
    let label = settings::label(&core.settings.lock().unwrap().shortcut, cfg!(target_os = "macos"));
    let text = if *core.shortcut_works.lock().unwrap() { format!("Open Clipframes  ({label})") } else { format!("Open Clipframes  ({label} is used by another app)") };
    if let Some(item) = core.open_item.lock().unwrap().as_ref() {
        let _ = item.set_text(text);
    };
}

/// Not a build run from the source tree: only an installed app adds itself to login.
fn installed() -> bool {
    std::env::current_exe().map(|p| !p.components().any(|c| c.as_os_str() == "target")).unwrap_or(false)
}

fn set_launch_at_login(app: &AppHandle, on: bool) {
    use tauri_plugin_autostart::ManagerExt;
    if !installed() {
        return;
    }
    let launcher = app.autolaunch();
    if let Err(e) = if on { launcher.enable() } else { launcher.disable() } {
        eprintln!("launch at login: {e}");
    }
}

/// What the login start is given on its command line: it is the one start that shows nothing.
const LOGIN_ARGS: &str = "--hidden";

/// Clipframes' entry among the programs Windows starts at login.
#[cfg(windows)]
mod login {
    use std::ffi::c_void;

    #[link(name = "advapi32")]
    extern "system" {
        fn RegGetValueW(key: *mut c_void, sub_key: *const u16, value: *const u16, flags: u32, kind: *mut u32, data: *mut c_void, size: *mut u32) -> i32;
        fn RegSetKeyValueW(key: *mut c_void, sub_key: *const u16, value: *const u16, kind: u32, data: *const c_void, size: u32) -> i32;
    }

    const ROOTS: [isize; 2] = [0x8000_0001u32 as i32 as isize, 0x8000_0002u32 as i32 as isize]; // current user, local machine
    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const RRF_RT_REG_SZ: u32 = 0x0000_0002;
    const REG_SZ: u32 = 1;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// Where the entry is and the command it holds, if there is one.
    fn find(name: &str) -> Option<(isize, String)> {
        let (run, name) = (wide(RUN), wide(name));
        ROOTS.into_iter().find_map(|root| {
            let mut text = [0u16; 2048];
            let mut bytes = (text.len() * 2) as u32;
            let read = unsafe { RegGetValueW(root as *mut c_void, run.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), text.as_mut_ptr().cast(), &mut bytes) };
            // Zero is success. The size counts bytes and the ending zero.
            (read == 0).then(|| (root, String::from_utf16_lossy(&text[..(bytes as usize / 2).min(text.len())]).trim_end_matches('\0').to_string()))
        })
    }

    /// The entry's command, or `None` when there is no entry.
    pub fn command(name: &str) -> Option<String> {
        find(name).map(|(_, command)| command)
    }

    /// Puts another command in the entry that is there, and touches nothing else: the on/off
    /// that Task Manager keeps beside it stays as the user left it.
    pub fn repoint(name: &str, command: &str) {
        if let Some((root, _)) = find(name) {
            let data = wide(command);
            unsafe { RegSetKeyValueW(root as *mut c_void, wide(RUN).as_ptr(), wide(name).as_ptr(), REG_SZ, data.as_ptr().cast(), (data.len() * 2) as u32) };
        }
    }
}

/// The state of the login entry where the system keeps an on/off of its own beside it
/// (Windows): `Some(None)` for no entry, `Some(Some(command))` for one. `None` elsewhere.
fn login_entry(_name: &str) -> Option<Option<String>> {
    #[cfg(windows)]
    return Some(login::command(_name));
    #[cfg(not(windows))]
    None
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Login {
    Add,
    Remove,
    /// The entry is there but starts a copy of the app somewhere else.
    Repoint,
    Leave,
}

/// What to do about the login entry when the app starts. On Windows, Task Manager and
/// Settings keep the user's own on/off beside the entry, and adding the entry again switches
/// it back on. So an entry that is there is never added again: it is only added when missing
/// (the first run, or an install after an uninstall) and removed when the setting says off.
/// One that is there but points at another place than the app that is running (an install
/// moved, a profile brought back from a backup) would start nothing: its command is put
/// right, and only that.
fn login_entry_at_start(wanted: bool, entry: Option<Option<&str>>, exe: &str) -> Login {
    match (wanted, entry) {
        (true, None) | (true, Some(None)) => Login::Add,
        (false, None) | (false, Some(Some(_))) => Login::Remove,
        (false, Some(None)) => Login::Leave,
        (true, Some(Some(command))) if starts(command, exe) => Login::Leave,
        (true, Some(Some(_))) => Login::Repoint,
    }
}

/// Whether a login entry's command starts this executable, however the path is written.
fn starts(command: &str, exe: &str) -> bool {
    let plain = |s: &str| s.replace(r"\\?\", "").replace('"', "").replace('/', "\\").to_lowercase();
    let (command, exe) = (plain(command), plain(exe));
    // The path, and then nothing or the arguments.
    !exe.is_empty() && command.strip_prefix(&exe).is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
}

/// Nothing of Clipframes is on screen: a safe moment to restart for an update.
pub fn idle(app: &AppHandle) -> bool {
    let visible = |label: &str| app.get_webview_window(label).is_some_and(|w| w.is_visible().unwrap_or(false));
    app.state::<Core>().picker.lock().unwrap().is_none() && !bar_visible(app) && !visible(SETTINGS) && !visible(HISTORY)
}

pub fn set_update_status(app: &AppHandle, status: String) {
    *app.state::<Core>().update.lock().unwrap() = status;
    let _ = app.emit("settings", settings_view(app));
}

fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    // As tall as what it shows: the list of places the tab appears in has room for three,
    // and scrolls after that. On a small screen the whole page scrolls instead.
    let core = app.state::<Core>();
    let listed = if core.settings.lock().unwrap().show_tab { core.places.lock().unwrap().shown(places::now()).len().min(3) } else { 0 };
    let wanted = if telemetry::available() { 734.0 } else { 629.0 } + if listed > 0 { 12.0 + 32.0 * listed as f64 } else { 0.0 };
    let room = app.primary_monitor().ok().flatten().map(|m| m.work_area().size.height as f64 / m.scale_factor() - 64.0);
    // An ordinary window, built when asked for and gone when closed.
    let built = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("index.html".into()))
        .title("Clipframes")
        .inner_size(440.0, room.map_or(wanted, |room| wanted.min(room)))
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .center()
        .background_color(tauri::window::Color(18, 18, 19, 255))
        .build();
    if let Ok(window) = built {
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn settings_get(app: AppHandle) -> SettingsView {
    settings_view(&app)
}

#[tauri::command]
fn shortcut_set(app: AppHandle, shortcut: String) -> Result<SettingsView, String> {
    if !settings::valid(&shortcut) {
        return Err("Use at least one of Ctrl, Alt or the system key, plus one other key.".into());
    }
    let core = app.state::<Core>();
    let old = core.settings.lock().unwrap().shortcut.clone();
    let had_old = *core.shortcut_works.lock().unwrap();
    if had_old {
        let _ = app.global_shortcut().unregister(old.as_str());
    }
    if bind_shortcut(&app, &shortcut).is_err() {
        // Keep what worked before.
        if had_old {
            let _ = bind_shortcut(&app, &old);
        }
        return Err(format!("{} is already used by another app. Try a different one.", settings::label(&shortcut, cfg!(target_os = "macos"))));
    }
    core.settings.lock().unwrap().shortcut = shortcut;
    *core.shortcut_works.lock().unwrap() = true;
    telemetry::event("shortcut_changed", json!({}));
    save_settings(&app);
    refresh_menu(&app);
    let _ = app.emit("round", view(&app));
    Ok(settings_view(&app))
}

#[tauri::command]
fn launch_set(app: AppHandle, on: bool) -> SettingsView {
    app.state::<Core>().settings.lock().unwrap().launch_at_login = on;
    save_settings(&app);
    set_launch_at_login(&app, on);
    settings_view(&app)
}

#[tauri::command]
fn usage_set(app: AppHandle, on: bool) -> SettingsView {
    app.state::<Core>().settings.lock().unwrap().share_usage = on;
    save_settings(&app);
    telemetry::set_enabled(on);
    settings_view(&app)
}

/// The switch for the tab everywhere. Off: the tab goes, and so does the watcher that looks
/// at which app is in front (`watch` ends by itself). On: it is started again.
#[tauri::command]
fn tab_set(app: AppHandle, on: bool) -> SettingsView {
    app.state::<Core>().settings.lock().unwrap().show_tab = on;
    save_settings(&app);
    telemetry::event("tab_set", json!({ "on": on }));
    if on {
        start_watch(&app);
    } else {
        hide_tab(&app);
    }
    // The pin in an open bar comes and goes with it.
    let _ = app.emit("round", view(&app));
    settings_view(&app)
}

/// Remove in the list of places: the tab is turned off there, as with the pin in the bar,
/// and stays off until the pin turns it on again.
#[tauri::command]
fn tab_place_remove(app: AppHandle, place: String, host: String) -> SettingsView {
    {
        let core = app.state::<Core>();
        let mut places = core.places.lock().unwrap();
        places.set_auto(&Place { app: place, host }, false, places::now());
        telemetry::event("place_auto_set", json!({ "on": false }));
        if let Some(dir) = config_dir(&app) {
            let _ = places.save(&dir);
        }
    }
    let _ = app.emit("round", view(&app));
    settings_view(&app)
}

/// Adds the rule for the captures folder to Claude Code's settings, or takes it away. What
/// comes back is what the file holds now, which is not `on` when it could not be changed.
#[tauri::command]
fn claude_read_set(app: AppHandle, on: bool) -> SettingsView {
    if let Some((file, rules)) = claude_read(&app) {
        let now = claude::set(&file, &rules, on);
        if now == (if on { claude::State::On } else { claude::State::Off }) {
            telemetry::event("claude_read_set", json!({ "on": on }));
        }
    }
    settings_view(&app)
}

/// Puts the rules on the clipboard as they are written in the file, for adding by hand.
#[tauri::command]
fn claude_rules_copy(app: AppHandle) {
    let Some((_, rules)) = claude_read(&app) else { return };
    let lines = rules.iter().map(|rule| json!(rule).to_string()).collect::<Vec<_>>().join(",\n");
    // Not through `copy_text`: this is not a round's text, and must not be taken back as one.
    let core = app.state::<Core>();
    let mut clipboard = core.clipboard.lock().unwrap();
    if clipboard.is_none() {
        *clipboard = arboard::Clipboard::new().ok();
    }
    if let Some(c) = clipboard.as_mut() {
        let _ = c.set_text(if cfg!(windows) { lines.replace('\n', "\r\n") } else { lines });
    }
}

/// Something went wrong in one of the windows' own code.
#[tauri::command]
fn ui_error(window: WebviewWindow, message: String, at: String) {
    telemetry::error("ui", &message, &format!("{}:{}", window.label(), at.rsplit('/').next().unwrap_or("")));
}

#[tauri::command]
fn update_check(app: AppHandle) {
    thread::spawn(move || {
        set_update_status(&app, "Checking…".into());
        let _ = updates::check(&app);
    });
}

pub fn run() {
    tauri::Builder::default()
        // Starting Clipframes while it is running opens the bar of the one that is.
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            // `clipframes --settings` opens Settings in the copy that is running.
            if args.iter().any(|a| a == "--settings") {
                return later(app, open_settings);
            }
            telemetry::via("launch");
            later(app, open)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // The login start says so, and that is the one start that shows nothing.
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec![LOGIN_ARGS])))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Core::default())
        // The system asking one of the round's windows to close (Alt+F4 on Windows) would
        // leave the round running without it. The window stays and the round ends properly.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let label = window.label();
                if label == BAR || label == NOTE || label.starts_with(OVERLAY) {
                    api.prevent_close();
                    later(window.app_handle(), close);
                }
            }
            if matches!(event, tauri::WindowEvent::Moved(_)) && window.label() == BAR {
                bar_moved(window.app_handle(), true);
            }
        })
        .invoke_handler(tauri::generate_handler![
            round_state,
            note_set,
            note_close,
            note_resize,
            pick_remove,
            round_done,
            escape_key,
            permission_open,
            app_quit,
            tool_set,
            recording_stop,
            history_open,
            history_list,
            history_copy,
            history_reveal,
            history_delete,
            place_auto_set,
            tab_open,
            bar_grip,
            bar_home,
            tab_set,
            tab_place_remove,
            settings_get,
            shortcut_set,
            launch_set,
            usage_set,
            claude_read_set,
            claude_rules_copy,
            ui_error,
            update_check
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let handle = app.handle().clone();
            let core = app.state::<Core>();
            if let Ok(home) = app.path().home_dir() {
                store::set_home(home);
            }

            let (saved, existed) = app.path().app_config_dir().map(|dir| settings::load(&dir)).unwrap_or_default();
            let mut saved = saved;
            let first_run = saved.install_id.is_empty();
            if first_run {
                saved.install_id = telemetry::new_install_id();
            }
            *core.settings.lock().unwrap() = saved.clone();
            if !existed || first_run {
                save_settings(&handle);
            }
            telemetry::start(saved.install_id.clone(), &app.package_info().version.to_string(), saved.share_usage);
            // Every start, so the login entry matches the setting even when the settings were
            // there before this copy was installed.
            let name = app.package_info().name.clone();
            let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
            match login_entry_at_start(saved.launch_at_login, login_entry(&name).as_ref().map(|e| e.as_deref()), &exe) {
                Login::Add => set_launch_at_login(&handle, true),
                Login::Remove => set_launch_at_login(&handle, false),
                // Written the way the entry is first made: the path, then the argument.
                #[cfg(windows)]
                Login::Repoint if installed() => login::repoint(&name, &format!("{exe} {LOGIN_ARGS}")),
                Login::Repoint | Login::Leave => {}
            }

            // Another app may own the shortcut already. Clipframes still runs: the tray opens
            // it, and Settings offers another shortcut.
            let bound = bind_shortcut(&handle, &saved.shortcut).inspect_err(|e| eprintln!("shortcut {} not registered: {e}", saved.shortcut)).is_ok();
            *core.shortcut_works.lock().unwrap() = bound;

            let open_item = MenuItem::with_id(app, "open", "Open Clipframes", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Clipframes", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &settings_item, &PredefinedMenuItem::separator(app)?, &quit_item])?;
            *core.open_item.lock().unwrap() = Some(open_item);
            refresh_menu(&handle);

            let tray = TrayIconBuilder::new().tooltip("Clipframes").menu(&menu).on_menu_event(|app, event| match event.id.as_ref() {
                "open" => {
                    telemetry::via("tray");
                    later(app, open)
                }
                "settings" => later(app, open_settings),
                "quit" => quit(app),
                _ => {}
            });
            // macOS menu bar icons are one colour and take the bar's own; elsewhere the app icon.
            #[cfg(target_os = "macos")]
            let tray = tray.icon(tauri::include_image!("icons/tray-mac.png")).icon_as_template(true);
            #[cfg(not(target_os = "macos"))]
            let tray = match app.default_window_icon() {
                Some(icon) => tray.icon(icon.clone()),
                None => tray,
            };
            tray.build(app)?;

            if let Some(dir) = config_dir(&handle) {
                *core.places.lock().unwrap() = Places::load(&dir);
            }
            start_watch(&handle);
            #[cfg(target_os = "macos")]
            thread::Builder::new().name("clipframes-tidy".into()).spawn(tidy)?;

            if installed() || std::env::var_os("CLIPFRAMES_UPDATES").is_some() {
                updates::start(&handle);
            }

            // Started by hand (search, the Start menu, a double click): show the bar. Started
            // at login or by an update: stay out of the way.
            #[cfg(all(feature = "selftest", target_os = "macos"))]
            if std::env::args().any(|a| a == "--selftest") {
                selftest::run(handle.clone());
                return Ok(());
            }
            let updated = updates::just_updated(&handle);
            let hidden = std::env::args().any(|a| a == LOGIN_ARGS);
            telemetry::event("app_started", json!({ "how": if updated { "update" } else if hidden { "login" } else { "hand" }, "first_run": first_run, "shortcut_works": bound, "launch_at_login": saved.launch_at_login }));
            let quiet = hidden | updated;
            if !quiet {
                later(&handle, if bound { open } else { open_settings });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Clipframes could not start")
        .run(|app, event| match event {
            // Closing the last window is not quitting: Clipframes lives in the tray.
            tauri::RunEvent::ExitRequested { api, code, .. } if code.is_none() => api.prevent_exit(),
            // macOS: the app was opened again while running (Spotlight, Launchpad, Finder).
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                telemetry::via("launch");
                later(app, open)
            }
            // Quit, an update's restart, the system shutting down: a comment still being
            // typed is written out first.
            tauri::RunEvent::Exit => {
                settle_note(app);
                // Apps asked for their page structure are not left with it on.
                element::sleep_idle(Duration::ZERO);
            }
            _ => {
                let _ = app;
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: Rect = Rect { x: 0.0, y: 0.0, width: 1920.0, height: 1080.0 };
    const LEFT: Rect = Rect { x: -1920.0, y: 0.0, width: 1920.0, height: 1080.0 };
    const ABOVE: Rect = Rect { x: 200.0, y: -1440.0, width: 2560.0, height: 1440.0 };

    #[test]
    fn a_point_is_on_the_display_that_holds_it_also_left_of_or_above_the_main_one() {
        let all = [MAIN, LEFT, ABOVE];
        assert_eq!(monitor_at(&all, 960.0, 540.0), Some(0));
        assert_eq!(monitor_at(&all, -1500.0, 300.0), Some(1));
        assert_eq!(monitor_at(&all, 1400.0, -700.0), Some(2));
        assert_eq!(monitor_at(&all, -0.5, 1079.5), Some(1));
    }

    #[test]
    fn a_point_on_an_edge_two_displays_share_is_on_the_one_that_begins_there() {
        let all = [MAIN, LEFT, ABOVE];
        assert_eq!(monitor_at(&all, 0.0, 500.0), Some(0), "the main display's left edge is its own");
        assert_eq!(monitor_at(&all, -1920.0, 0.0), Some(1), "a display's top-left corner is its own");
        assert_eq!(monitor_at(&all, 500.0, 0.0), Some(0), "the top edge of the main one, not the bottom of the one above");
        assert_eq!(monitor_at(&all, 500.0, -0.01), Some(2));
        assert_eq!(monitor_at(&all, 1920.0, 500.0), None, "a right edge belongs to no one when nothing is beyond it");
        assert_eq!(monitor_at(&all, 500.0, 1080.0), None);
    }

    #[test]
    fn a_point_on_no_display_is_on_none() {
        let all = [MAIN, LEFT, ABOVE];
        assert_eq!(monitor_at(&all, 5000.0, 100.0), None);
        assert_eq!(monitor_at(&all, -100.0, -100.0), None, "in the corner between two displays");
        assert_eq!(monitor_at(&[], 0.0, 0.0), None);
        assert_eq!(monitor_at(&all, f64::NAN, 0.0), None);
    }

    /// The bar on a display with a 25-unit menu bar and a 70-unit Dock.
    const WORK: Rect = Rect { x: 0.0, y: 25.0, width: 1920.0, height: 985.0 };
    const BAR_ON_IT: (f64, f64) = (396.0, 64.0);

    #[test]
    fn a_dragged_bar_comes_back_to_the_same_place() {
        let at = (40.0, 300.0);
        let kept = fraction(&WORK, BAR_ON_IT, at);
        let back = placed(&WORK, BAR_ON_IT, kept);
        assert!((back.0 - at.0).abs() < 0.001 && (back.1 - at.1).abs() < 0.001, "{back:?}");
        assert_eq!(fraction(&WORK, BAR_ON_IT, (WORK.x, WORK.y)), (0.0, 0.0), "the top-left corner of the free room");
        assert_eq!(fraction(&WORK, BAR_ON_IT, (1920.0 - 396.0, 1010.0 - 64.0)), (1.0, 1.0), "and the bottom-right one");
    }

    #[test]
    fn the_bars_place_holds_when_the_display_changes_size_or_scale() {
        // Against the right edge, a third of the way down.
        let kept = fraction(&WORK, BAR_ON_IT, (1524.0, 340.0));
        // The same display at a lower resolution.
        let small = Rect { x: 0.0, y: 25.0, width: 1280.0, height: 625.0 };
        let (x, y) = placed(&small, BAR_ON_IT, kept);
        assert_eq!(x, 1280.0 - 396.0, "still against the right edge");
        assert!(y > 200.0 && y < 230.0, "still a third of the way down: {y}");
        // And where one unit is two pixels: the bar is twice as large, like the room it is in.
        let dense = Rect { x: 3840.0, y: 50.0, width: 3840.0, height: 1970.0 };
        let (x, y) = placed(&dense, (792.0, 128.0), kept);
        assert_eq!((x, y), (3840.0 + 3048.0, 680.0));
    }

    #[test]
    fn a_bar_let_go_partly_off_the_display_is_remembered_fully_on_it() {
        for at in [(-200.0, 500.0), (1800.0, 500.0), (700.0, -30.0), (700.0, 1060.0), (5000.0, 5000.0)] {
            let (x, y) = placed(&WORK, BAR_ON_IT, fraction(&WORK, BAR_ON_IT, at));
            assert!(x >= WORK.x && x + BAR_ON_IT.0 <= WORK.x + WORK.width, "{at:?} came back at x {x}");
            assert!(y >= WORK.y && y + BAR_ON_IT.1 <= WORK.y + WORK.height, "{at:?} came back at y {y}");
        }
        // Under the menu bar is not free room either.
        assert_eq!(placed(&WORK, BAR_ON_IT, fraction(&WORK, BAR_ON_IT, (700.0, 0.0))).1, 25.0);
    }

    #[test]
    fn a_place_that_makes_no_sense_still_puts_the_bar_on_the_display() {
        // A settings file edited by hand, or written by something else.
        for kept in [(7.0, -3.0), (f64::NAN, f64::INFINITY)] {
            let (x, y) = placed(&WORK, BAR_ON_IT, kept);
            assert!(x >= WORK.x && x + BAR_ON_IT.0 <= WORK.x + WORK.width && y >= WORK.y && y + BAR_ON_IT.1 <= WORK.y + WORK.height, "{kept:?} gave {x}, {y}");
        }
        // No room at all: the middle, and a number that can be written to the file.
        let tiny = Rect { x: 0.0, y: 0.0, width: 300.0, height: 40.0 };
        assert_eq!(fraction(&tiny, BAR_ON_IT, (10.0, 10.0)), (0.5, 0.5));
        assert_eq!(placed(&tiny, BAR_ON_IT, (0.0, 1.0)), (-48.0, -12.0));
        assert_eq!(fraction(&WORK, BAR_ON_IT, (f64::NAN, 300.0)).0, 0.5);
    }

    #[test]
    fn a_dragged_bar_belongs_to_the_display_showing_most_of_it() {
        let all = [MAIN, LEFT, ABOVE];
        assert_eq!(display_showing(&Rect { x: 700.0, y: 900.0, width: 396.0, height: 64.0 }, &all), Some(0));
        assert_eq!(display_showing(&Rect { x: -100.0, y: 900.0, width: 396.0, height: 64.0 }, &all), Some(0), "296 of its 396 are on the main one");
        assert_eq!(display_showing(&Rect { x: -300.0, y: 900.0, width: 396.0, height: 64.0 }, &all), Some(1));
        assert_eq!(display_showing(&Rect { x: 1800.0, y: 1050.0, width: 396.0, height: 64.0 }, &all), Some(0), "hanging off the corner, but only this one shows it");
        assert_eq!(display_showing(&Rect { x: 5000.0, y: 900.0, width: 396.0, height: 64.0 }, &all), None);
    }

    #[test]
    fn two_displays_of_the_same_model_are_told_apart_by_where_they_are() {
        let named = |name: &str, frame: Rect| (name.to_string(), frame);
        let desk = [named("Monitor #1", MAIN), named("Monitor #7", LEFT), named("Monitor #7", ABOVE)];
        let ids = display_ids(&desk);
        assert_eq!(ids, ["Monitor #1", "Monitor #7", "Monitor #7 (2)"], "the left one first");
        // Listed in another order, each is still called the same.
        let again = display_ids(&[desk[2].clone(), desk[0].clone(), desk[1].clone()]);
        assert_eq!(again, ["Monitor #7 (2)", "Monitor #1", "Monitor #7"]);
        assert_eq!(arrangement(&ids), arrangement(&again));
        assert_eq!(arrangement(&ids), "Monitor #1 + Monitor #7 + Monitor #7 (2)");
        // The laptop alone is another arrangement, with its own place for the bar.
        assert_ne!(arrangement(&display_ids(&desk[..1])), arrangement(&ids));
        assert_eq!(display_ids(&[named("", MAIN)]), ["Display"], "a display the system has no name for");
    }

    // A display twice as dense as a unit, with a taskbar along its bottom.
    const SCREEN: Rect = Rect { x: 0.0, y: 0.0, width: 2560.0, height: 1440.0 };
    const FREE: Rect = Rect { x: 0.0, y: 0.0, width: 2560.0, height: 1380.0 };
    const HOME: Rect = Rect { x: 876.0, y: 1120.0, width: 808.0, height: 128.0 };
    const SIDE: f64 = 88.0;

    #[test]
    fn the_tab_place_and_the_note_style_come_from_their_names_and_default_to_today() {
        assert_eq!(["", "bar", "Edge", " under ", "NEAREST", "elsewhere"].map(tab_place_named), [TabPlace::Bar, TabPlace::Bar, TabPlace::Edge, TabPlace::Under, TabPlace::Nearest, TabPlace::Bar]);
        assert_eq!(["", "box", "Line", " aside ", "YIELD", "other"].map(note_style_named), [NoteStyle::Box, NoteStyle::Box, NoteStyle::Line, NoteStyle::Aside, NoteStyle::Yield, NoteStyle::Box]);
    }

    #[test]
    fn with_the_bar_never_moved_the_tab_is_at_bottom_centre_whatever_the_way() {
        let middle = (1280.0 - 44.0, 1184.0 - 44.0);
        for place in [TabPlace::Bar, TabPlace::Edge, TabPlace::Under] {
            assert_eq!(tab_at(place, &SCREEN, &FREE, &HOME, &HOME, SIDE), middle, "{place:?}");
        }
        // The nearest edge is the bottom one, where the taskbar is: whole, against it.
        assert_eq!(tab_at(TabPlace::Nearest, &SCREEN, &FREE, &HOME, &HOME, SIDE), (1236.0, 1380.0 - 88.0));
    }

    #[test]
    fn with_the_bar_moved_each_way_puts_the_tab_somewhere_else() {
        // The bar dragged to the upper left of the middle of the screen.
        let bar = Rect { x: 100.0, y: 500.0, ..HOME };
        assert_eq!(tab_at(TabPlace::Bar, &SCREEN, &FREE, &bar, &HOME, SIDE), (504.0 - 44.0, 564.0 - 44.0), "in the middle of the bar");
        assert_eq!(tab_at(TabPlace::Edge, &SCREEN, &FREE, &bar, &HOME, SIDE), (1236.0, 1140.0), "where it always was");
        assert_eq!(tab_at(TabPlace::Under, &SCREEN, &FREE, &bar, &HOME, SIDE), (460.0, 1140.0), "at the bottom, under the bar");
        assert_eq!(tab_at(TabPlace::Nearest, &SCREEN, &FREE, &bar, &HOME, SIDE), (-44.0, 520.0), "half past the left edge, level with the bar");
    }

    #[test]
    fn the_tab_at_the_nearest_edge_is_half_hidden_only_where_the_display_ends() {
        let near = |x: f64, y: f64| tab_at(TabPlace::Nearest, &SCREEN, &FREE, &Rect { x, y, ..HOME }, &HOME, SIDE);
        assert_eq!(near(900.0, 10.0), (1260.0, -44.0), "the top edge of this display has no bar of the system's");
        assert_eq!(near(1740.0, 600.0), (2560.0 - 44.0, 620.0), "the right edge");
        assert_eq!(near(900.0, 1240.0), (1260.0, 1292.0), "at the taskbar it stays whole");
        // In a corner it does not hang past the end of the edge it is on.
        assert_eq!(near(0.0, 0.0), (360.0, -44.0));
        assert_eq!(near(1752.0, 0.0).0, 2112.0);
        // A menu bar along the top: whole, under it.
        let under_menu = Rect { x: 0.0, y: 50.0, width: 2560.0, height: 1390.0 };
        assert_eq!(tab_at(TabPlace::Nearest, &SCREEN, &under_menu, &Rect { x: 900.0, y: 60.0, ..HOME }, &HOME, SIDE), (1260.0, 50.0));
    }

    #[test]
    fn the_tab_under_a_bar_at_the_side_of_the_display_stays_on_it() {
        let hanging = Rect { x: -700.0, y: 400.0, ..HOME };
        assert_eq!(tab_at(TabPlace::Under, &SCREEN, &FREE, &hanging, &HOME, SIDE).0, 0.0);
        let hanging = Rect { x: 2400.0, y: 400.0, ..HOME };
        assert_eq!(tab_at(TabPlace::Under, &SCREEN, &FREE, &hanging, &HOME, SIDE).0, 2560.0 - 88.0);
        // On a display left of the main one everything is counted from its own corner.
        let (screen, free, home) = (Rect { x: -1920.0, ..MAIN }, Rect { x: -1920.0, height: 1040.0, ..MAIN }, Rect { x: -1162.0, y: 920.0, width: 404.0, height: 64.0 });
        assert_eq!(tab_at(TabPlace::Edge, &screen, &free, &home, &home, 44.0), (-982.0, 930.0));
        assert_eq!(tab_at(TabPlace::Nearest, &screen, &free, &Rect { x: -1900.0, y: 300.0, ..home }, &home, 44.0), (-1942.0, 310.0));
    }

    const NOTE: (f64, f64) = (316.0, 172.0);
    const BUTTON: Rect = Rect { x: 600.0, y: 300.0, width: 120.0, height: 40.0 };

    fn spot(style: NoteStyle, pick: &Rect) -> (f64, f64) {
        note_spot(style, &MAIN, pick, NOTE, 8.0, false)
    }

    fn on_screen(at: (f64, f64), size: (f64, f64), screen: &Rect) -> bool {
        at.0 >= screen.x && at.1 >= screen.y && at.0 + size.0 <= screen.x + screen.width && at.1 + size.1 <= screen.y + screen.height
    }

    #[test]
    fn the_comment_box_opens_under_the_pick_or_over_it_when_there_is_no_room() {
        assert_eq!(spot(NoteStyle::Box, &BUTTON), (600.0, 348.0));
        assert_eq!(spot(NoteStyle::Yield, &BUTTON), (600.0, 348.0), "the yielding box starts out the same");
        assert_eq!(spot(NoteStyle::Box, &Rect { y: 1000.0, ..BUTTON }), (600.0, 1000.0 - 172.0 - 8.0), "at the bottom of the display: over it");
        assert_eq!(spot(NoteStyle::Box, &Rect { x: 1850.0, ..BUTTON }).0, 1920.0 - 316.0 - 8.0, "at the right edge: pulled in");
        assert_eq!(spot(NoteStyle::Box, &Rect { x: -40.0, ..BUTTON }).0, 8.0);
        // A pick as tall as the display: on the pick, at the top, as it has always been.
        assert_eq!(spot(NoteStyle::Box, &Rect { x: 200.0, y: 0.0, width: 900.0, height: 1080.0 }), (200.0, 8.0));
    }

    #[test]
    fn the_one_line_box_fits_under_picks_the_tall_one_has_to_go_over() {
        let low = Rect { y: 960.0, ..BUTTON };
        assert_eq!(note_spot(NoteStyle::Line, &MAIN, &low, (316.0, 60.0), 8.0, false), (600.0, 1008.0));
        assert_eq!(spot(NoteStyle::Box, &low), (600.0, 780.0));
        // Grown to its three lines there it no longer fits under, and goes over.
        assert_eq!(note_spot(NoteStyle::Line, &MAIN, &low, (316.0, 100.0), 8.0, false), (600.0, 852.0));
    }

    #[test]
    fn the_box_beside_the_pick_goes_right_then_left_then_under_then_over() {
        assert_eq!(spot(NoteStyle::Aside, &BUTTON), (728.0, 300.0), "right of it, top to top");
        assert_eq!(spot(NoteStyle::Aside, &Rect { x: 1700.0, ..BUTTON }), (1700.0 - 316.0 - 8.0, 300.0), "no room on the right: left");
        // A row across the whole display: neither side, so under it.
        let row = Rect { x: 0.0, y: 300.0, width: 1920.0, height: 40.0 };
        assert_eq!(spot(NoteStyle::Aside, &row), (8.0, 348.0));
        assert_eq!(spot(NoteStyle::Aside, &Rect { y: 1000.0, ..row }), (8.0, 820.0), "and over it at the bottom");
        // A tall column in the middle: beside it, and kept on the display at the bottom.
        let column = Rect { x: 800.0, y: 0.0, width: 300.0, height: 1080.0 };
        assert_eq!(spot(NoteStyle::Aside, &column), (1108.0, 8.0));
        assert_eq!(spot(NoteStyle::Aside, &Rect { y: 1000.0, ..BUTTON }), (728.0, 1080.0 - 172.0 - 8.0), "beside a pick at the bottom: pulled up");
        // Something as large as the display: nowhere beside it, so as the plain box does.
        assert_eq!(spot(NoteStyle::Aside, &MAIN), spot(NoteStyle::Box, &MAIN));
    }

    #[test]
    fn the_yielding_box_steps_over_the_pick_or_beside_it() {
        let stepped = |pick: &Rect| note_spot(NoteStyle::Yield, &MAIN, pick, NOTE, 8.0, true);
        assert_eq!(stepped(&BUTTON), (600.0, 300.0 - 172.0 - 8.0), "from under the pick to over it");
        assert_eq!(stepped(&Rect { y: 60.0, ..BUTTON }), (728.0, 60.0), "no room over it: to the right");
        assert_eq!(stepped(&Rect { x: 1700.0, y: 60.0, ..BUTTON }), (1376.0, 60.0), "or the left");
        // Already over the pick, at the bottom of the display: beside it.
        assert_eq!(stepped(&Rect { y: 1000.0, ..BUTTON }), (728.0, 900.0));
        // Nowhere else to go: it stays.
        assert_eq!(stepped(&MAIN), spot(NoteStyle::Box, &MAIN));
        // The other styles never step.
        assert_eq!(note_spot(NoteStyle::Box, &MAIN, &BUTTON, NOTE, 8.0, true), spot(NoteStyle::Box, &BUTTON));
    }

    #[test]
    fn the_comment_box_is_on_the_display_wherever_the_pick_is_and_whatever_the_style() {
        let small = Rect { x: -1280.0, y: 200.0, width: 1280.0, height: 720.0 };
        let picks = [
            BUTTON,
            Rect { x: 0.0, y: 0.0, width: 20.0, height: 20.0 },
            Rect { x: 1900.0, y: 1060.0, width: 20.0, height: 20.0 },
            Rect { x: -300.0, y: -200.0, width: 4000.0, height: 3000.0 },
            Rect { x: 900.0, y: -50.0, width: 30.0, height: 5000.0 },
            Rect { x: -50.0, y: 500.0, width: 5000.0, height: 30.0 },
        ];
        for screen in [MAIN, small] {
            for pick in picks {
                let pick = Rect { x: pick.x + screen.x, y: pick.y + screen.y, ..pick };
                for (style, size) in [(NoteStyle::Box, NOTE), (NoteStyle::Line, (316.0, 60.0)), (NoteStyle::Line, (316.0, 100.0)), (NoteStyle::Aside, NOTE), (NoteStyle::Yield, NOTE)] {
                    for stepped in [false, true] {
                        let at = note_spot(style, &screen, &pick, size, 8.0, stepped);
                        assert!(on_screen(at, size, &screen), "{style:?} (stepped {stepped}) for {pick:?} on {screen:?} came out at {at:?}");
                    }
                }
            }
        }
        // A display smaller than the box: its top-left corner is what stays in sight.
        let tiny = Rect { x: 0.0, y: 0.0, width: 300.0, height: 150.0 };
        for style in [NoteStyle::Box, NoteStyle::Aside, NoteStyle::Yield] {
            assert_eq!(note_spot(style, &tiny, &Rect { x: 100.0, y: 50.0, width: 40.0, height: 20.0 }, NOTE, 8.0, true), (8.0, 8.0), "{style:?}");
        }
    }

    #[test]
    fn the_yielding_box_steps_aside_for_a_pointer_that_comes_for_it_not_one_that_wobbles() {
        let note = Rect { x: 600.0, y: 348.0, width: 316.0, height: 172.0 };
        assert_eq!(away(&note, 650.0, 320.0), 28.0);
        assert_eq!(away(&note, 650.0, 400.0), 0.0, "inside");
        assert_eq!(away(&note, 597.0, 344.0), 5.0, "off a corner");
        // On the button it was opened for, 28 away. A few pixels of wobble is not coming for it.
        assert!(!comes_for(28.0, 28.0));
        assert!(!comes_for(28.0, 22.0));
        assert!(comes_for(28.0, 14.0), "moved well towards it, and nearly there");
        // From far away it counts once it is close.
        assert!(!comes_for(400.0, 80.0));
        assert!(comes_for(400.0, 20.0));
    }

    #[test]
    fn the_switch_in_settings_stops_the_place_watcher() {
        assert_eq!(watch_turn(true, false), Watch::Look);
        assert_eq!(watch_turn(true, true), Watch::Wait, "during a round no other app is asked anything");
        assert_eq!(watch_turn(false, false), Watch::Stop);
        assert_eq!(watch_turn(false, true), Watch::Stop, "also in the middle of a round");
    }

    #[test]
    fn an_element_on_a_display_left_of_the_main_one_is_pictured_where_it_is() {
        let around = Rect { x: -1518.0, y: 182.0, width: 136.0, height: 76.0 };
        assert_eq!(on_display(&around, &[MAIN, LEFT]), Some(around));
        // At the very edge of that display the padding is cut off, not moved to the main one.
        let edge = Rect { x: -1938.0, y: -18.0, width: 136.0, height: 76.0 };
        assert_eq!(on_display(&edge, &[MAIN, LEFT]), Some(Rect { x: -1920.0, y: 0.0, width: 118.0, height: 58.0 }));
    }

    #[test]
    fn an_element_far_larger_than_the_screen_is_pictured_only_where_it_shows() {
        let page = Rect { x: 200.0, y: -3000.0, width: 1400.0, height: 40000.0 };
        assert_eq!(on_display(&page, &[MAIN, LEFT]), Some(Rect { x: 200.0, y: 0.0, width: 1400.0, height: 1080.0 }));
    }

    #[test]
    fn an_element_across_two_displays_is_pictured_on_the_one_showing_more_of_it() {
        let across = Rect { x: -100.0, y: 100.0, width: 400.0, height: 50.0 };
        assert_eq!(on_display(&across, &[MAIN, LEFT]), Some(Rect { x: 0.0, y: 100.0, width: 300.0, height: 50.0 }));
    }

    #[test]
    fn the_clipboard_is_only_ours_while_it_holds_what_we_put_there() {
        assert!(same_text("[Clipframes: 2 things]\r\n1. Button\r\n2. Link", "[Clipframes: 2 things]\n1. Button\n2. Link"));
        assert!(!same_text("something the user copied since", "[Button \"Save\"]"));
        assert!(!same_text("", ""), "an empty clipboard needs no emptying");
    }

    #[test]
    fn comment_text_that_arrives_out_of_order_does_not_replace_newer_text() {
        let newest = AtomicU64::new(0);
        assert!(newer(&newest, 1_760_000_000_001));
        assert!(newer(&newest, 1_760_000_000_003));
        assert!(!newer(&newest, 1_760_000_000_002), "the keystroke before, arriving after");
        assert!(!newer(&newest, 1_760_000_000_003), "the same one twice");
        assert!(newer(&newest, 1_760_000_000_004));
    }

    #[test]
    fn what_is_learned_after_a_round_closed_reaches_the_clipboard_only_if_it_is_still_ours() {
        assert_eq!(late(true, false), Late::Publish);
        assert_eq!(late(true, true), Late::Publish);
        assert_eq!(late(false, true), Late::SaveAndCopy);
        assert_eq!(late(false, false), Late::Save, "the user has copied something else since");
    }

    #[test]
    fn a_new_round_counts_comment_messages_from_one_again() {
        let newest = AtomicU64::new(0);
        for seq in 1..=40 {
            assert!(newer(&newest, seq));
        }
        assert!(!newer(&newest, 1), "within the round an old number stays old");
        // What opening a round does. Without it every keystroke of the next round would be
        // taken for an old one.
        newest.store(0, Ordering::SeqCst);
        assert!(newer(&newest, 1));
        assert!(newer(&newest, 2));
    }

    const HERE: &str = r"C:\Users\sam\AppData\Local\Clipframes\clipframes.exe";

    #[test]
    fn a_login_entry_switched_off_in_task_manager_is_not_written_again() {
        let ours = format!("{HERE} --hidden");
        // Windows: the entry is there, and whether it is on is the user's business.
        assert_eq!(login_entry_at_start(true, Some(Some(&ours)), HERE), Login::Leave);
        // First run, or installed again after an uninstall: add it.
        assert_eq!(login_entry_at_start(true, Some(None), HERE), Login::Add);
        // The setting says off: take it away, and do nothing when it is already gone.
        assert_eq!(login_entry_at_start(false, Some(Some(&ours)), HERE), Login::Remove);
        assert_eq!(login_entry_at_start(false, Some(None), HERE), Login::Leave);
        // Elsewhere the entry simply follows the setting.
        assert_eq!(login_entry_at_start(true, None, HERE), Login::Add);
        assert_eq!(login_entry_at_start(false, None, HERE), Login::Remove);
    }

    #[test]
    fn a_login_entry_that_starts_a_copy_somewhere_else_is_pointed_at_this_one() {
        assert_eq!(login_entry_at_start(true, Some(Some(r"D:\Old\Clipframes\clipframes.exe --hidden")), HERE), Login::Repoint);
        // A longer name that begins the same is another program.
        assert_eq!(login_entry_at_start(true, Some(Some(r"C:\Users\sam\AppData\Local\Clipframes\clipframes.exe.old --hidden")), HERE), Login::Repoint);
        // The same place written another way is this one.
        for same in [r#""C:\Users\sam\AppData\Local\Clipframes\clipframes.exe" --hidden"#, r"c:\users\SAM\appdata\local\clipframes\CLIPFRAMES.EXE --hidden", r"\\?\C:\Users\sam\AppData\Local\Clipframes\clipframes.exe", "C:/Users/sam/AppData/Local/Clipframes/clipframes.exe --hidden"] {
            assert_eq!(login_entry_at_start(true, Some(Some(same)), HERE), Login::Leave, "{same}");
        }
        // With the setting off it is removed, wherever it points.
        assert_eq!(login_entry_at_start(false, Some(Some(r"D:\Old\clipframes.exe")), HERE), Login::Remove);
    }

    #[test]
    fn an_element_on_no_display_gets_no_picture() {
        assert_eq!(on_display(&Rect { x: 5000.0, y: 5000.0, width: 100.0, height: 100.0 }, &[MAIN, LEFT]), None);
        assert_eq!(on_display(&MAIN, &[]), None);
    }

    #[test]
    fn the_shortcut_opens_when_nothing_is_running_and_closes_otherwise() {
        assert_eq!(turn(true, false, false), Turn::Open);
        assert_eq!(turn(true, true, true), Turn::Close);
        // The bar showing a message, with no round behind it.
        assert_eq!(turn(true, false, true), Turn::Close);
    }

    #[test]
    fn the_shortcut_ends_a_round_whose_bar_is_not_on_screen() {
        assert_eq!(turn(true, true, false), Turn::Close);
    }

    #[test]
    fn asking_to_open_leaves_an_open_round_alone_and_ends_one_with_no_bar() {
        assert_eq!(turn(false, false, false), Turn::Open);
        assert_eq!(turn(false, false, true), Turn::Open);
        assert_eq!(turn(false, true, true), Turn::Stay);
        assert_eq!(turn(false, true, false), Turn::Close);
    }
}
