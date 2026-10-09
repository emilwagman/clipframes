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
use crate::picker::{Event, Picker};
use crate::round::Round;
use crate::settings::{self, Settings};
use crate::store::{self, Stamp};
use crate::updates;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const BAR: &str = "bar";
const NOTE: &str = "note";
const SETTINGS: &str = "settings";
/// How long the hidden bar is kept after closing, ready to open again at once.
const KEEP_WARM: Duration = Duration::from_secs(90);

/// Sizes in CSS pixels.
const BAR_SIZE: (f64, f64) = (640.0, 80.0);
const NOTE_SIZE: (f64, f64) = (380.0, 158.0);

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
    settings: Mutex<Settings>,
    /// False when another app owns the shortcut, so Clipframes could not take it.
    shortcut_works: Mutex<bool>,
    /// The tray's "Open" line, which shows the shortcut.
    open_item: Mutex<Option<MenuItem<tauri::Wry>>>,
    /// The last thing the updater had to say.
    update: Mutex<String>,
    /// Why the round could not start, for a bar that loads after the fact.
    trouble: Mutex<Option<String>>,
    /// Counts opens and closes, so a keep-warm timer knows if it is out of date.
    turn: AtomicU64,
    /// When the bar was asked to open, to time how long until it can draw.
    opened: Mutex<Option<Instant>>,
}

/// What the bar and the comment box draw.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoundView {
    picking: bool,
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
struct PickView {
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
}

fn view(app: &AppHandle) -> RoundView {
    let core = app.state::<Core>();
    let round = core.round.lock().unwrap();
    let picking = core.picker.lock().unwrap().is_some();
    let noting = *core.noting.lock().unwrap();
    let trouble = core.trouble.lock().unwrap().clone();
    let state = RoundView {
        picking,
        picks: round.picks.iter().map(|p| PickView { headline: p.element.headline(), selector: p.element.selector(), note: p.note.clone() }).collect(),
        noting,
        reference: round.reference(core.notes.lock().unwrap().as_deref().and_then(|p| p.to_str())),
        trouble,
        shortcut: settings::label(&core.settings.lock().unwrap().shortcut, cfg!(target_os = "macos")),
    };
    state
}

/// Tells every window what the round looks like now, and puts it on the clipboard.
fn publish(app: &AppHandle) {
    let core = app.state::<Core>();
    save(&core);
    let state = view(app);
    if !state.reference.is_empty() {
        let mut clipboard = core.clipboard.lock().unwrap();
        if clipboard.is_none() {
            *clipboard = arboard::Clipboard::new().ok();
        }
        if let Some(c) = clipboard.as_mut() {
            // Windows programs expect CRLF; a terminal there joins lines that end in a bare LF.
            let text = if cfg!(windows) { state.reference.replace('\n', "\r\n") } else { state.reference.clone() };
            let _ = c.set_text(text);
        }
    }
    let _ = app.emit("round", &state);

    let frames: Vec<Rect> = core.round.lock().unwrap().picks.iter().map(|p| p.element.frame).collect();
    for screen in core.screens.lock().unwrap().iter() {
        let marks: Vec<MarkView> = frames.iter().enumerate().map(|(i, f)| MarkView { number: i + 1, rect: screen.local(f) }).collect();
        let _ = app.emit_to(screen.label.as_str(), "marks", marks);
    }
}

/// Writes the round to its folder, making the folder at the first pick.
fn save(core: &Core) {
    let round = core.round.lock().unwrap();
    if round.picks.is_empty() {
        return;
    }
    let mut folder = core.folder.lock().unwrap();
    let (path, taken) = folder.get_or_insert_with(|| {
        let taken = Stamp::now();
        (store::new_folder(&store::root(), taken), taken)
    });
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
    core.turn.fetch_add(1, Ordering::SeqCst);
    *core.opened.lock().unwrap() = Some(started);
    *core.round.lock().unwrap() = Round::default();
    *core.noting.lock().unwrap() = None;
    *core.shown.lock().unwrap() = None;
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
    refresh_exempt(app);
    publish(app);
    eprintln!("open ({}): picking after {:.0} ms, all windows after {:.0} ms", if was_warm { "warm" } else { "cold" }, picking.as_secs_f64() * 1000.0, started.elapsed().as_secs_f64() * 1000.0);
}

/// Ends the round and hides everything. What was picked stays on the clipboard.
fn close(app: &AppHandle) {
    trace("close: begin");
    let core = app.state::<Core>();
    // Taken out first and dropped unlocked: stopping the picker waits for its threads.
    let picker = core.picker.lock().unwrap().take();
    trace("close: picker taken");
    drop(picker);
    trace("close: picker stopped");
    *core.noting.lock().unwrap() = None;
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
            let index = core.round.lock().unwrap().add(element);
            *core.noting.lock().unwrap() = Some(index);
            show_note(app, &frame);
            publish(app);
        }
        Event::Cancel => {
            trace("esc");
            later(app, escape)
        }
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

/// Esc closes the comment box if one is open, and the round otherwise.
fn escape(app: &AppHandle) {
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
    app.state::<Core>().round.lock().unwrap().remove(index);
    publish(&app);
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

/// Opens the system's page for the permission Clipframes needs to read other apps.
#[tauri::command]
fn permission_open(app: AppHandle) {
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().open_url("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility", None::<&str>);
}

/// What the settings window draws.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsView {
    shortcut: String,
    shortcut_label: String,
    shortcut_works: bool,
    launch_at_login: bool,
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
    app.state::<Core>().picker.lock().unwrap().is_none() && !visible(BAR) && !visible(SETTINGS)
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
fn update_check(app: AppHandle) {
    thread::spawn(move || {
        set_update_status(&app, "Checking…".into());
        let _ = updates::check(&app);
    });
}

pub fn run() {
    tauri::Builder::default()
        // Starting Clipframes while it is running opens the bar of the one that is.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| later(app, open)))
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
            settings_get,
            shortcut_set,
            launch_set,
            update_check
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let handle = app.handle().clone();
            let core = app.state::<Core>();

            let (saved, existed) = app.path().app_config_dir().map(|dir| settings::load(&dir)).unwrap_or_default();
            *core.settings.lock().unwrap() = saved.clone();
            if !existed {
                save_settings(&handle);
                set_launch_at_login(&handle, saved.launch_at_login);
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
                "open" => later(app, open),
                "settings" => later(app, open_settings),
                "quit" => app.exit(0),
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

            if installed() || std::env::var_os("CLIPFRAMES_UPDATES").is_some() {
                updates::start(&handle);
            }

            // Started by hand (search, the Start menu, a double click): show the bar. Started
            // at login or by an update: stay out of the way.
            let quiet = std::env::args().any(|a| a == "--hidden") | updates::just_updated(&handle);
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
            tauri::RunEvent::Reopen { .. } => later(app, open),
            _ => {
                let _ = app;
            }
        });
}
