//! The app: a tray icon, one shortcut, the bar, and a round of picks.
//!
//! Nothing is open while Clipframes is idle. A web view costs a few hundred megabytes on
//! Windows even when hidden, so every window is built when the bar opens. The overlays, the
//! comment box and the input source go away when it closes; the bar stays hidden for a short
//! while, so opening it again straight away is instant, and then it goes too.
//!
//! Positions come in "picker units": what the system's input and element APIs report. That is
//! points on macOS and physical pixels on Windows, so every conversion to a window lives here.

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
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg(all(feature = "selftest", target_os = "macos"))]
#[path = "selftest.rs"]
mod selftest;

const BAR: &str = "bar";
const NOTE: &str = "note";
const SETTINGS: &str = "settings";
const HISTORY: &str = "history";
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

/// Sizes in CSS pixels.
const BAR_SIZE: (f64, f64) = (376.0, 64.0);
const NOTE_SIZE: (f64, f64) = (316.0, 172.0);

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
    round: Mutex<Round>,
    picker: Mutex<Option<Picker>>,
    screens: Mutex<Vec<Screen>>,
    /// The pick whose comment box is open.
    noting: Mutex<Option<usize>>,
    /// The highlight the overlays are showing, so an unchanged answer is not sent again.
    shown: Mutex<Option<Rect>>,
    /// Kept for the whole run: on Linux the clipboard's content lives only as long as its owner.
    clipboard: Mutex<Option<arboard::Clipboard>>,
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
    /// Counts opens and closes, so a keep-warm timer knows if it is out of date.
    turn: AtomicU64,
    /// When the bar was asked to open, to time how long until it can draw.
    opened: Mutex<Option<Instant>>,
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
        place: core.over.lock().unwrap().as_ref().filter(|p| p.known()).map(|p| PlaceView { name: p.name(), auto: core.places.lock().unwrap().auto(p) }),
        picks: round.picks.iter().map(|p| PickView { kind: p.kind, headline: p.headline(), selector: if p.kind == Kind::Element { p.element.selector() } else { String::new() }, note: p.note.clone() }).collect(),
        noting,
        reference: round.reference(core.notes.lock().unwrap().as_deref().and_then(|p| p.to_str())),
        trouble,
        shortcut: settings::label(&core.settings.lock().unwrap().shortcut, cfg!(target_os = "macos")),
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
    }
}

/// Tells every window what the round looks like now, and puts it on the clipboard.
fn publish(app: &AppHandle) {
    let core = app.state::<Core>();
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

/// Where the tab sits: in the middle of where the bar opens. Physical pixels.
fn tab_position(app: &AppHandle, side: f64) -> Option<PhysicalPosition<i32>> {
    let (bar, scale) = bar_position(app)?;
    Some(PhysicalPosition::new(bar.x + ((BAR_SIZE.0 * scale - side) / 2.0) as i32, bar.y + ((BAR_SIZE.1 * scale - side) / 2.0) as i32))
}

#[cfg(windows)]
fn show_tab(app: &AppHandle) {
    let core = app.state::<Core>();
    let mut tab = core.tab.lock().unwrap();
    if tab.is_none() {
        let handle = app.clone();
        *tab = tab::Tab::start(protected(), move || later(&handle, open));
    }
    if let Some(tab) = tab.as_ref() {
        if let Some(at) = tab_position(app, tab.side() as f64) {
            tab.show(at.x, at.y);
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
fn show_tab(app: &AppHandle) {
    let window = app.get_webview_window(TAB).or_else(|| small_window(app, TAB, (tab::SIZE + 12.0, tab::SIZE + 12.0)).build().ok());
    let Some(window) = window else { return };
    let scale = window.scale_factor().unwrap_or(1.0);
    if let Some(at) = tab_position(app, (tab::SIZE + 12.0) * scale) {
        let _ = window.set_position(at);
    }
    let _ = window.show();
}

#[cfg(not(windows))]
fn hide_tab(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(TAB) {
        let _ = window.hide();
    }
}

/// Once a second: which app or site is in front, and whether the tab belongs there. Reading
/// the page address is the only part that costs anything, and it happens only when the
/// window in front has changed.
fn watch(app: AppHandle) {
    let mut seen: Option<(i32, String)> = None;
    let mut read_at = Instant::now();
    let mut showing = false;
    loop {
        thread::sleep(Duration::from_secs(1));
        let core = app.state::<Core>();
        if core.picker.lock().unwrap().is_some() {
            if showing {
                hide_tab(&app);
                showing = false;
            }
            continue;
        }
        let Some(front) = element::foreground() else { continue };
        let key = (front.pid, front.title.clone());
        if seen.as_ref() != Some(&key) {
            if seen.is_some() && read_at.elapsed() < Duration::from_millis(1500) {
                continue; // titles that change all the time do not get a reading each
            }
            let url = if element::permitted() { element::element_full_at(front.frame.x + front.frame.width / 2.0, front.frame.y + front.frame.height / 2.0).map(|e| e.url).unwrap_or_default() } else { String::new() };
            *core.front.lock().unwrap() = Some(Place::new(&front.app, &url));
            seen = Some(key);
            read_at = Instant::now();
        }
        let wanted = core.front.lock().unwrap().as_ref().is_some_and(|p| core.places.lock().unwrap().wants(p, places::now()));
        if wanted != showing {
            if wanted { show_tab(&app) } else { hide_tab(&app) }
            showing = wanted;
        }
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

/// Where the bar goes: bottom centre of the main display, in physical pixels.
fn bar_position(app: &AppHandle) -> Option<(PhysicalPosition<i32>, f64)> {
    let m = app.primary_monitor().ok()??;
    let (pos, size, scale) = (m.position(), m.size(), m.scale_factor());
    let (w, h) = (BAR_SIZE.0 * scale, BAR_SIZE.1 * scale);
    let x = pos.x as f64 + (size.width as f64 - w) / 2.0;
    let y = pos.y as f64 + size.height as f64 - h - 96.0 * scale;
    Some((PhysicalPosition::new(x as i32, y as i32), scale))
}

/// The bar, on screen. A kept one is moved and shown; a new one is built where it belongs and
/// already visible, so nothing waits for a second step once the web view is up.
fn show_bar(app: &AppHandle) -> Option<WebviewWindow> {
    let place = bar_position(app);
    if let Some(bar) = app.get_webview_window(BAR) {
        if let Some((position, _)) = place {
            let _ = bar.set_position(position);
        }
        let _ = bar.show();
        return Some(bar);
    }
    let mut builder = small_window(app, BAR, BAR_SIZE).visible(true);
    if let Some((position, scale)) = place {
        builder = builder.position(position.x as f64 / scale, position.y as f64 / scale);
    }
    builder.build().ok()
}

/// One click-through window per display. They only paint; the picker owns the input.
fn open_overlays(app: &AppHandle) -> Vec<Screen> {
    let mut screens = Vec::new();
    for (i, m) in app.available_monitors().unwrap_or_default().iter().enumerate() {
        let label = format!("overlay-{i}");
        let built = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
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
            .build();
        let Ok(w) = built else { continue };
        let (pos, size, scale) = (*m.position(), *m.size(), m.scale_factor());
        let _ = w.set_position(PhysicalPosition::new(pos.x, pos.y));
        let _ = w.set_size(PhysicalSize::new(size.width, size.height));
        let _ = w.set_ignore_cursor_events(true);
        let _ = w.show();
        // macOS reports points, everything else physical pixels.
        let unit = if cfg!(target_os = "macos") { scale } else { 1.0 };
        screens.push(Screen {
            label,
            frame: Rect { x: pos.x as f64 / unit, y: pos.y as f64 / unit, width: size.width as f64 / unit, height: size.height as f64 / unit },
            css: unit / scale,
        });
    }
    screens
}

fn close_round_windows(app: &AppHandle) {
    let core = app.state::<Core>();
    for screen in core.screens.lock().unwrap().drain(..) {
        if let Some(w) = app.get_webview_window(&screen.label) {
            let _ = w.destroy();
        }
    }
    if let Some(w) = app.get_webview_window(NOTE) {
        let _ = w.destroy();
    }
}

/// Opens the bar and starts picking. Call from a thread that may wait: it builds windows.
fn open(app: &AppHandle) {
    let core = app.state::<Core>();
    if core.picker.lock().unwrap().is_some() {
        return;
    }
    trace("open: begin");
    let started = Instant::now();
    let via = telemetry::take_via();
    core.turn.fetch_add(1, Ordering::SeqCst);
    *core.opened.lock().unwrap() = Some(started);
    *core.began.lock().unwrap() = Some(started);
    *core.round.lock().unwrap() = Round::default();
    *core.noting.lock().unwrap() = None;
    *core.shown.lock().unwrap() = None;
    *core.tool.lock().unwrap() = Kind::Element;
    let front = core.front.lock().unwrap().clone();
    *core.over.lock().unwrap() = front;
    hide_tab(app);
    core.files.store(0, Ordering::SeqCst);
    *core.folder.lock().unwrap() = None;
    *core.notes.lock().unwrap() = None;
    *core.trouble.lock().unwrap() = (!element::permitted()).then(|| "permission".to_string());

    let was_warm = app.get_webview_window(BAR).is_some();
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
    let _ = small_window(app, NOTE, NOTE_SIZE).build();
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

/// Ends the round and hides everything. What was picked stays on the clipboard.
fn close(app: &AppHandle) {
    trace("close: begin");
    let core = app.state::<Core>();
    let was_open = core.picker.lock().unwrap().is_some();
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
    if was_open {
        let round = core.round.lock().unwrap();
        let count = |kind: Kind| round.picks.iter().filter(|p| p.kind == kind).count();
        let seconds = core.began.lock().unwrap().take().map(|t| t.elapsed().as_secs()).unwrap_or(0);
        telemetry::event("round_closed", json!({ "picks": round.picks.len(), "elements": count(Kind::Element), "areas": count(Kind::Area), "clips": count(Kind::Clip), "notes": round.picks.iter().filter(|p| !p.note.is_empty()).count(), "seconds": seconds }));
    }
    close_round_windows(app);
    trace("close: windows closed");
    if let Some(bar) = app.get_webview_window(BAR) {
        let _ = bar.hide();
    }
    let _ = app.emit("round", view(app));

    // Let the bar go too, unless it is opened again first.
    let turn = core.turn.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(KEEP_WARM);
        if app.state::<Core>().turn.load(Ordering::SeqCst) == turn {
            if let Some(bar) = app.get_webview_window(BAR) {
                let _ = bar.destroy();
            }
        }
    });
}

fn toggle(app: &AppHandle) {
    let open_now = app.get_webview_window(BAR).is_some_and(|w| w.is_visible().unwrap_or(false));
    if open_now { close(app) } else { open(app) }
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
        Event::Hover { element, .. } => {
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
        Event::Pick { element, .. } => {
            trace("pick");
            let frame = element.frame;
            // A picture of the element with a little space around it. Not having one is fine.
            let pad = if cfg!(windows) { 18.0 } else { 12.0 };
            let around = Rect { x: (frame.x - pad).max(0.0), y: (frame.y - pad).max(0.0), width: frame.width + pad * 2.0, height: frame.height + pad * 2.0 };
            let (image, pixels) = snap(&core, &around, ELEMENT_WIDTH).unwrap_or_default();
            telemetry::event("pick_added", json!({ "kind": "element", "picture": !image.is_empty(), "selector": !element.selector().is_empty(), "named": !element.name.is_empty(), "web": !element.url.is_empty() }));
            remember(app, &element);
            let index = core.round.lock().unwrap().push(Pick { element, image, pixels, ..Default::default() });
            *core.noting.lock().unwrap() = Some(index);
            show_note(app, &frame);
            publish(app);
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
    let core = app.state::<Core>();
    telemetry::error("capture", "the screen could not be captured", "app::cannot_capture");
    *core.trouble.lock().unwrap() = Some(if cfg!(target_os = "macos") { "screen".into() } else { "The screen could not be captured.".into() });
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
        loop {
            let at = recording.started.elapsed();
            if recording.stop.load(Ordering::SeqCst) || at > LONGEST_CLIP {
                break;
            }
            match shot::capture_to_file(&rect, &dir.join(format!("{:03}.png", frames + 1)), Some(FRAME_WIDTH)) {
                Ok(size) => {
                    frames += 1;
                    pixels = size;
                    times.push(at.as_secs_f64());
                }
                Err(_) if frames == 0 => break,
                Err(_) => {}
            }
            if frames % 4 == 0 {
                let _ = app.emit("round", view(&app)); // the clock in the bar
            }
            let next = FRAME_EVERY * frames.max(1);
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

/// Puts the comment box under the picked element, inside its display.
fn show_note(app: &AppHandle, frame: &Rect) {
    let core = app.state::<Core>();
    let Some(note) = app.get_webview_window(NOTE) else { return };
    let screens = core.screens.lock().unwrap();
    let Some(screen) = screens.iter().find(|s| s.contains(frame.x + frame.width / 2.0, frame.y + frame.height / 2.0)).or(screens.first()) else { return };
    let (w, h, gap) = (NOTE_SIZE.0 / screen.css, NOTE_SIZE.1 / screen.css, 8.0 / screen.css);
    let s = &screen.frame;
    let x = frame.x.clamp(s.x + gap, (s.x + s.width - w - gap).max(s.x + gap));
    let below = frame.y + frame.height + gap;
    let y = if below + h <= s.y + s.height - gap { below } else { (frame.y - h - gap).max(s.y + gap) };
    drop(screens);
    let _ = if cfg!(target_os = "macos") {
        note.set_position(tauri::LogicalPosition::new(x, y))
    } else {
        note.set_position(PhysicalPosition::new(x as i32, y as i32))
    };
    let _ = note.show();
    let _ = note.set_focus();
    refresh_exempt(app);
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

#[tauri::command]
fn note_set(app: AppHandle, index: usize, note: String) {
    app.state::<Core>().round.lock().unwrap().set_note(index, &note);
    publish(&app);
}

#[tauri::command]
fn note_close(app: AppHandle) {
    hide_note(&app);
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
        copy_text(&app, "");
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

/// Opens the system's page for a permission Clipframes needs.
#[tauri::command]
fn permission_open(app: AppHandle, kind: String) {
    use tauri_plugin_opener::OpenerExt;
    let pane = if kind == "screen" { "Privacy_ScreenCapture" } else { "Privacy_Accessibility" };
    let _ = app.opener().open_url(format!("x-apple.systempreferences:com.apple.preference.security?{pane}"), None::<&str>);
    *app.state::<Core>().trouble.lock().unwrap() = None;
}

#[tauri::command]
fn tool_set(app: AppHandle, tool: Kind) {
    let core = app.state::<Core>();
    if core.recording.lock().unwrap().is_some() {
        return;
    }
    *core.tool.lock().unwrap() = tool;
    *core.trouble.lock().unwrap() = None;
    if let Some(picker) = core.picker.lock().unwrap().as_ref() {
        picker.set_mode(if tool == Kind::Element { Mode::Element } else { Mode::Area });
    }
    // The element highlight belongs to the element tool.
    *core.shown.lock().unwrap() = None;
    for screen in core.screens.lock().unwrap().iter() {
        let _ = app.emit_to(screen.label.as_str(), "hover", HoverView { rect: None, label: String::new() });
    }
    let _ = app.emit("round", view(&app));
}

#[tauri::command]
fn recording_stop(app: AppHandle) {
    stop_recording(&app);
}

#[derive(Debug, Clone, Serialize)]
struct HistoryPage {
    entries: Vec<store::Entry>,
    total: usize,
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
fn history_list(from: usize, count: usize) -> HistoryPage {
    let (entries, total) = store::list(&store::root(), from, count.min(200));
    HistoryPage { entries, total }
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
}

fn settings_view(app: &AppHandle) -> SettingsView {
    let core = app.state::<Core>();
    let settings = core.settings.lock().unwrap().clone();
    let shortcut_works = *core.shortcut_works.lock().unwrap();
    let update = core.update.lock().unwrap().clone();
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

/// Nothing of Clipframes is on screen: a safe moment to restart for an update.
pub fn idle(app: &AppHandle) -> bool {
    let visible = |label: &str| app.get_webview_window(label).is_some_and(|w| w.is_visible().unwrap_or(false));
    app.state::<Core>().picker.lock().unwrap().is_none() && !visible(BAR) && !visible(SETTINGS) && !visible(HISTORY)
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
    // An ordinary window, built when asked for and gone when closed.
    let built = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("index.html".into()))
        .title("Clipframes")
        .inner_size(440.0, 420.0)
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
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Core::default())
        .invoke_handler(tauri::generate_handler![
            round_state,
            note_set,
            note_close,
            pick_remove,
            round_done,
            escape_key,
            permission_open,
            tool_set,
            recording_stop,
            history_open,
            history_list,
            history_copy,
            history_reveal,
            history_delete,
            place_auto_set,
            tab_open,
            settings_get,
            shortcut_set,
            launch_set,
            usage_set,
            ui_error,
            update_check
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let handle = app.handle().clone();
            let core = app.state::<Core>();

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
            set_launch_at_login(&handle, saved.launch_at_login);

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
                "quit" => {
                    telemetry::flush();
                    app.exit(0)
                }
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
            let watcher = handle.clone();
            thread::Builder::new().name("clipframes-watch".into()).spawn(move || watch(watcher))?;

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
            let hidden = std::env::args().any(|a| a == "--hidden");
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
            _ => {
                let _ = app;
            }
        });
}
