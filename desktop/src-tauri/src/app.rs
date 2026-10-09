//! The app: a tray icon, one shortcut, the bar, and a round of picks.
//!
//! Nothing is open while Clipframes is idle: the bar is a hidden window, and the overlays, the
//! comment box and the input source exist only between opening the bar and closing it.
//!
//! Positions come in "picker units": what the system's input and element APIs report. That is
//! points on macOS and physical pixels on Windows, so every conversion to a window lives here.

use crate::element::{self, ElementInfo, Rect};
use crate::picker::{Event, Picker};
use crate::round::Round;
use serde::Serialize;
use std::sync::Mutex;
use std::thread;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const BAR: &str = "bar";
const NOTE: &str = "note";
const SHORTCUT: &str = "ctrl+shift+space";

/// Sizes in CSS pixels.
const BAR_SIZE: (f64, f64) = (372.0, 76.0);
const NOTE_SIZE: (f64, f64) = (320.0, 104.0);

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
}

#[derive(Debug, Clone, Serialize)]
struct PickView {
    headline: String,
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

fn view(app: &AppHandle, trouble: Option<String>) -> RoundView {
    let core = app.state::<Core>();
    let round = core.round.lock().unwrap();
    let picking = core.picker.lock().unwrap().is_some();
    let noting = *core.noting.lock().unwrap();
    let state = RoundView {
        picking,
        picks: round.picks.iter().map(|p| PickView { headline: p.element.headline(), note: p.note.clone() }).collect(),
        noting,
        reference: round.reference(None),
        trouble,
    };
    state
}

/// Tells every window what the round looks like now, and puts it on the clipboard.
fn publish(app: &AppHandle) {
    let core = app.state::<Core>();
    let state = view(app, None);
    if !state.reference.is_empty() {
        let mut clipboard = core.clipboard.lock().unwrap();
        if clipboard.is_none() {
            *clipboard = arboard::Clipboard::new().ok();
        }
        if let Some(c) = clipboard.as_mut() {
            let _ = c.set_text(state.reference.clone());
        }
    }
    let _ = app.emit("round", &state);

    let frames: Vec<Rect> = core.round.lock().unwrap().picks.iter().map(|p| p.element.frame).collect();
    for screen in core.screens.lock().unwrap().iter() {
        let marks: Vec<MarkView> = frames.iter().enumerate().map(|(i, f)| MarkView { number: i + 1, rect: screen.local(f) }).collect();
        let _ = app.emit_to(screen.label.as_str(), "marks", marks);
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

fn small_window(app: &AppHandle, label: &str, size: (f64, f64)) -> tauri::Result<WebviewWindow> {
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
        .build()
}

/// Bottom centre of the main display.
fn place_bar(app: &AppHandle, bar: &WebviewWindow) {
    let Ok(Some(m)) = app.primary_monitor() else { return };
    let (pos, size, scale) = (m.position(), m.size(), m.scale_factor());
    let (w, h) = (BAR_SIZE.0 * scale, BAR_SIZE.1 * scale);
    let x = pos.x as f64 + (size.width as f64 - w) / 2.0;
    let y = pos.y as f64 + size.height as f64 - h - 96.0 * scale;
    let _ = bar.set_position(PhysicalPosition::new(x as i32, y as i32));
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
    let Some(bar) = app.get_webview_window(BAR) else { return };
    place_bar(app, &bar);
    let _ = bar.show();

    *core.round.lock().unwrap() = Round::default();
    *core.noting.lock().unwrap() = None;
    *core.shown.lock().unwrap() = None;

    if !element::permitted() {
        let _ = app.emit("round", view(app, Some("permission".into())));
        return;
    }
    *core.screens.lock().unwrap() = open_overlays(app);
    let _ = small_window(app, NOTE, NOTE_SIZE);

    let handle = app.clone();
    match Picker::start(move |event| on_event(&handle, event)) {
        Ok(picker) => {
            *core.picker.lock().unwrap() = Some(picker);
            refresh_exempt(app);
            publish(app);
        }
        Err(message) => {
            close_round_windows(app);
            let _ = app.emit("round", view(app, Some(message)));
        }
    }
}

/// Ends the round and hides everything. What was picked stays on the clipboard.
fn close(app: &AppHandle) {
    let core = app.state::<Core>();
    // Taken out first and dropped unlocked: stopping the picker waits for its threads.
    let picker = core.picker.lock().unwrap().take();
    drop(picker);
    *core.noting.lock().unwrap() = None;
    close_round_windows(app);
    if let Some(bar) = app.get_webview_window(BAR) {
        let _ = bar.hide();
    }
    let _ = app.emit("round", view(app, None));
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
            let frame = element.frame;
            let index = core.round.lock().unwrap().add(element);
            *core.noting.lock().unwrap() = Some(index);
            show_note(app, &frame);
            publish(app);
        }
        // Esc closes the comment box if one is open, and the round otherwise.
        Event::Cancel => later(app, |app| if !hide_note(app) { close(app) }),
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

/// True if a comment box was open.
fn hide_note(app: &AppHandle) -> bool {
    let core = app.state::<Core>();
    let was_open = core.noting.lock().unwrap().take().is_some();
    if was_open {
        if let Some(note) = app.get_webview_window(NOTE) {
            let _ = note.hide();
        }
        refresh_exempt(app);
        publish(app);
    }
    was_open
}

#[tauri::command]
fn round_state(app: AppHandle) -> RoundView {
    view(&app, None)
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
fn round_done(app: AppHandle) {
    later(&app, close);
}

/// Opens the system's page for the permission Clipframes needs to read other apps.
#[tauri::command]
fn permission_open(app: AppHandle) {
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().open_url("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility", None::<&str>);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(Core::default())
        .invoke_handler(tauri::generate_handler![round_state, note_set, note_close, pick_remove, round_done, permission_open])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            small_window(app.handle(), BAR, BAR_SIZE)?;

            // Another app may own the shortcut already. Clipframes still runs: the tray opens it.
            let taken = app
                .global_shortcut()
                .on_shortcut(SHORTCUT, |app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        later(app, toggle);
                    }
                })
                .inspect_err(|e| eprintln!("shortcut {SHORTCUT} not registered: {e}"))
                .is_err();

            let open_label = if taken { "Open Clipframes (shortcut in use by another app)" } else { "Open Clipframes" };
            let open_item = MenuItem::with_id(app, "open", open_label, true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Clipframes", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &quit_item])?;
            let mut tray = TrayIconBuilder::new().tooltip("Clipframes").menu(&menu).on_menu_event(|app, event| match event.id.as_ref() {
                "open" => later(app, open),
                "quit" => app.exit(0),
                _ => {}
            });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            // `clipframes --open` starts with the bar open: for tests and for a second launch.
            if std::env::args().any(|a| a == "--open") {
                later(app.handle(), open);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Clipframes could not start")
        // Closing the last window is not quitting: Clipframes lives in the tray.
        .run(|_, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
