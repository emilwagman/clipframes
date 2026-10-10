//! A walk through every tool with real pointer events, for a Mac nobody is sitting at.
//!
//! Built only with `--features selftest` and started with `--selftest`; it is never part of a
//! release. The app posts the pointer events itself (it holds the Accessibility permission),
//! so they travel the same road as a hand on the mouse: through the system, into the picker.
//! Every step prints what the round looks like to stderr.
//!
//! With CLIPFRAMES_SELFTEST_PLACEMENT set it checks instead what a hand would have to: the
//! comment box that lets the pointer through, moving the bar by its grip, and the tab
//! (`placement`). Each check prints a line that begins "selftest PASS" or "selftest FAIL".

use super::*;
use core_graphics::display::CGDisplay;
use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton, EventField};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

fn post(kind: CGEventType, x: f64, y: f64) {
    let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else { return };
    if let Ok(event) = CGEvent::new_mouse_event(source, kind, CGPoint::new(x, y), CGMouseButton::Left) {
        event.post(CGEventTapLocation::HID);
    }
}

/// A press or release that says which click of a double click it is.
fn post_click(kind: CGEventType, x: f64, y: f64, nth: i64) {
    let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else { return };
    if let Ok(event) = CGEvent::new_mouse_event(source, kind, CGPoint::new(x, y), CGMouseButton::Left) {
        event.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, nth);
        event.post(CGEventTapLocation::HID);
    }
}

/// A key pressed and let go, sent to whatever has the keyboard.
fn key(code: u16) {
    for down in [true, false] {
        let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else { return };
        if let Ok(event) = CGEvent::new_keyboard_event(source, code, down) {
            event.post(CGEventTapLocation::HID);
        }
        wait(30);
    }
}

fn click(x: f64, y: f64) {
    post(CGEventType::LeftMouseDown, x, y);
    wait(60);
    post(CGEventType::LeftMouseUp, x, y);
}

/// What the run is doing now, for the watchdog to say if it never gets further.
static STEP: Mutex<String> = Mutex::new(String::new());
static DONE: AtomicBool = AtomicBool::new(false);
/// The settings and places files as they were before the run, to be put back.
static FILES: Mutex<Vec<(PathBuf, Option<Vec<u8>>)>> = Mutex::new(Vec::new());

fn step(what: &str) {
    *STEP.lock().unwrap() = what.to_string();
}

fn restore_files() {
    for (path, bytes) in FILES.lock().unwrap().drain(..) {
        let _ = match bytes {
            Some(bytes) => std::fs::write(&path, bytes),
            None => std::fs::remove_file(&path),
        };
    }
}

/// A run that has not finished after `longest` says where it was, puts the files back and
/// ends the app: a self-test that hangs must not sit on the machine with the pointer taken.
fn watchdog(longest: Duration) {
    thread::spawn(move || {
        thread::sleep(longest);
        if DONE.load(Ordering::SeqCst) {
            return;
        }
        // Not waiting for the lock: the run may be stuck holding it.
        let at = STEP.try_lock().map(|s| s.clone()).unwrap_or_else(|_| "(not known)".into());
        eprintln!("selftest FAIL watchdog: not finished after {} s, last step: {at}", longest.as_secs());
        if let Ok(mut files) = FILES.try_lock() {
            for (path, bytes) in files.drain(..) {
                let _ = match bytes {
                    Some(bytes) => std::fs::write(&path, bytes),
                    None => std::fs::remove_file(&path),
                };
            }
        }
        eprintln!("selftest stopped by the watchdog");
        std::process::exit(2);
    });
}

/// One line per check, and the count for the last line.
struct Checks {
    passed: u32,
    failed: u32,
}

impl Checks {
    fn check(&mut self, what: &str, ok: bool, seen: String) {
        step(&format!("after the check \"{what}\""));
        if ok {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
        eprintln!("selftest {} {what}: {seen}", if ok { "PASS" } else { "FAIL" });
    }
}

fn middle_of(r: &Rect) -> (f64, f64) {
    (r.x + r.width / 2.0, r.y + r.height / 2.0)
}

/// Where the bar's grip is: at the start of the card, which sits 8 in from the window's edge.
fn grip_of(bar: &Rect) -> (f64, f64) {
    (bar.x + 21.0, bar.y + bar.height / 2.0)
}

/// The place remembered for the bar on the display that shows this point.
fn bar_place(app: &AppHandle, near: (f64, f64)) -> Option<(f64, f64)> {
    let shown = display_near(app, Some(near))?;
    app.state::<Core>().settings.lock().unwrap().bar_places.get(&shown.arrangement).and_then(|places| places.get(&shown.id)).copied()
}

/// The checks for the comment box, the bar's grip and the tab. Settings and remembered places
/// are put back as they were, in memory and on disk, whatever happens on the way. The picks
/// it makes are real: they are saved among the captures and copied, as in the walk-through.
fn placement(app: &AppHandle, w: f64, h: f64) {
    let core = app.state::<Core>();
    let mut checks = Checks { passed: 0, failed: 0 };
    let dir = config_dir(app);
    *FILES.lock().unwrap() = dir.iter().flat_map(|dir| ["settings.json", "places.json"].map(|name| dir.join(name))).map(|path| (path.clone(), std::fs::read(&path).ok())).collect();
    let (settings_before, places_before) = (core.settings.lock().unwrap().clone(), core.places.lock().unwrap().clone());
    // From a known start: the bar where it opens by default, the tab switched on.
    {
        let mut settings = core.settings.lock().unwrap();
        settings.bar_places.clear();
        settings.show_tab = true;
    }
    start_watch(app);
    eprintln!("selftest placement: comment style {:?}, tab place {:?}", note_style(), tab_place());

    placement_steps(app, w, h, &mut checks);

    step("putting settings and places back");
    let picking = core.picker.lock().unwrap().is_some();
    if picking || bar_visible(app) {
        close(app);
    }
    *core.settings.lock().unwrap() = settings_before;
    *core.places.lock().unwrap() = places_before;
    restore_files();
    if core.settings.lock().unwrap().show_tab {
        start_watch(app);
    }
    eprintln!("selftest placement: settings and places are as they were before the run");
    eprintln!("selftest placement: {} passed, {} failed", checks.passed, checks.failed);
}

fn placement_steps(app: &AppHandle, w: f64, h: f64, checks: &mut Checks) {
    let core = app.state::<Core>();
    let picks = || view(app).picks;
    let target = (w * 0.5, h * 0.42);

    // 1. The comment box lets the pointer through.
    step("1 the first pick");
    glide(CGEventType::MouseMoved, (w * 0.3, h * 0.3), target);
    wait(500);
    click(target.0, target.1);
    wait(1800);
    checks.check("1 a click is a pick", picks().len() == 1, format!("{} picked: {:?}", picks().len(), picks().first().map(|p| p.headline.clone())));
    if picks().len() != 1 {
        return eprintln!("selftest placement stopped: without a pick there is no comment box to check");
    }
    if note_style() != NoteStyle::Ghost {
        eprintln!("selftest placement: the comment box that lets the pointer through is not the one in use (CLIPFRAMES_NOTE_STYLE), its checks are left out");
    } else {
        let note = app.get_webview_window(NOTE).filter(|n| n.is_visible().unwrap_or(false)).and_then(|n| window_rect(&n));
        let Some(note) = note else {
            return checks.check("1 the comment box is on screen", false, "no visible comment box window".into());
        };
        eprintln!("selftest placement: comment box at {note:?}, for the pick at {:?}", core.noted.lock().unwrap().as_ref().map(|n| n.pick));
        // Keys first, while the pointer is nowhere near: typing goes to the box.
        step("1 typing in the comment box");
        for _ in 0..3 {
            key(0); // the A key
        }
        wait(500);
        let typed = core.round.lock().unwrap().picks.first().map(|p| p.note.clone()).unwrap_or_default();
        checks.check("1 typing reaches the comment", typed == "aaa", format!("three A keys were pressed, the comment reads {typed:?}"));

        step("1 the pointer on the comment box");
        let on_box = middle_of(&note);
        post(CGEventType::MouseMoved, on_box.0, on_box.1);
        wait(400);
        // What the core told the box's window. What the window then draws is not read here.
        checks.check("1 the box goes faint under a resting pointer", core.faint.load(Ordering::SeqCst), format!("pointer at {on_box:?}, faint {}", core.faint.load(Ordering::SeqCst)));
        step("1 the click on the comment box");
        click(on_box.0, on_box.1);
        wait(1800);
        let under = element::element_at(on_box.0, on_box.1).map(|e| e.headline()).unwrap_or_default();
        let second = picks().get(1).map(|p| p.headline.clone());
        checks.check("1 a click on the box picks what is under it", picks().len() == 2 && second.as_deref().is_some_and(|s| !s.is_empty()), format!("{} picked, the second is {second:?}; read at that point now: {under:?}", picks().len()));
        // Read into a value of its own first: a lock taken twice in one statement waits for
        // itself for ever, which is what this line once did.
        let kept = core.round.lock().unwrap().picks.first().map(|p| p.note.clone());
        checks.check("1 the first comment was kept", kept.as_deref() == Some(typed.as_str()), format!("{kept:?}"));
        post(CGEventType::MouseMoved, target.0, target.1);
        wait(400);
        checks.check("1 the box is no longer faint with the pointer off it", !core.faint.load(Ordering::SeqCst), format!("faint {}", core.faint.load(Ordering::SeqCst)));
    }
    step("1 closing the comment box");
    hide_note(app);
    wait(400);

    // 2. The bar is moved by its grip.
    step("2 dragging the bar by its grip");
    let Some(before) = app.get_webview_window(BAR).and_then(|b| window_rect(&b)) else {
        return checks.check("2 the bar is on screen", false, "no bar window".into());
    };
    let grip = grip_of(&before);
    let to = (grip.0 - 300.0, grip.1 - 200.0);
    eprintln!("selftest placement: bar at {before:?}, remembered {:?}; dragging its grip from {grip:?} to {to:?}", bar_place(app, grip));
    post(CGEventType::MouseMoved, grip.0, grip.1);
    wait(300);
    post(CGEventType::LeftMouseDown, grip.0, grip.1);
    wait(250);
    glide(CGEventType::LeftMouseDragged, grip, to);
    wait(100);
    post(CGEventType::LeftMouseUp, to.0, to.1);
    wait(900);
    let after = app.get_webview_window(BAR).and_then(|b| window_rect(&b)).unwrap_or(before);
    let moved = (after.x - before.x, after.y - before.y);
    let dragged = (moved.0 + 300.0).abs() < 12.0 && (moved.1 + 200.0).abs() < 12.0;
    checks.check("2 the grip drags the bar", dragged, format!("the bar moved by {moved:?} (asked: -300, -200), now at {after:?}"));
    if !dragged && moved == (0.0, 0.0) {
        eprintln!("selftest placement: the bar did not move at all: the system's own drag did not follow posted pointer events, so the drag itself is not checked by this run");
    }
    let remembered = bar_place(app, middle_of(&after));
    checks.check("2 the new place is remembered", dragged && remembered.is_some(), format!("barPlaces has {remembered:?} for this display"));

    // Straight to a click somewhere else, with no move in between: the pick must be of
    // what is there (once the first pick after a drag was the page around it).
    step("2 the click right after the drag");
    let count = picks().len();
    click(target.0, target.1);
    wait(1800);
    let got = picks().get(count).map(|p| p.headline.clone());
    let there = element::element_at(target.0, target.1).map(|e| e.headline()).unwrap_or_default();
    checks.check("2 a click right after the drag picks what is clicked", picks().len() == count + 1 && got.as_deref() == Some(there.as_str()), format!("picked {got:?}, read at that point: {there:?}"));
    step("2 closing the round");
    hide_note(app);
    close(app);
    wait(1200);
    step("3 waiting for the tab");

    // 3. The tab, with the bar's place moved: at the bottom, under where the bar would open.
    // The place in front is made a remembered one, in memory only.
    wait(2500);
    let front = core.front.lock().unwrap().clone();
    eprintln!("selftest placement: in front {:?}, as a place {front:?}", element::foreground().map(|f| (f.app, f.frame)));
    let Some(front) = front.filter(Place::known) else {
        return checks.check("3 the watcher knows what is in front", false, "nothing in front that is not Clipframes".into());
    };
    core.places.lock().unwrap().used(&front, places::now());
    wait(2500);
    let tab = || app.get_webview_window(TAB).filter(|t| t.is_visible().unwrap_or(false)).and_then(|t| window_rect(&t));
    checks.check("3 the tab shows over a remembered place", tab().is_some(), format!("tab window {:?}", tab()));
    if let (Some(tab), Some(window)) = (tab(), element::foreground()) {
        // The tab goes to the display its window is on. Everything here in points.
        let near = middle_of(&window.frame);
        let bar = bar_spot(app, Some(near)).map(|s| (s.at.x as f64 / s.scale + BAR_SIZE.0 / 2.0, s.at.y as f64 / s.scale + BAR_SIZE.1 / 2.0));
        let bottom = display_near(app, Some(near)).map(|d| (home_rect(&d.monitor), d.monitor.scale_factor())).map(|(home, scale)| (home.y + home.height / 2.0) / scale);
        let at = middle_of(&tab);
        let under = bar.is_some_and(|bar| (at.0 - bar.0).abs() < 3.0) && bottom.is_some_and(|y| (at.1 - y).abs() < 3.0);
        checks.check("3 the tab is at the bottom, under the bar's place", under || tab_place() != TabPlace::Under, format!("tab middle {at:?}; the bar would open with its middle at {bar:?}; the bottom row is at y {bottom:?}"));
    }
    step("3 the switch for the tab, off");
    tab_set(app.clone(), false);
    wait(1800);
    checks.check("3 the switch off hides the tab", tab().is_none(), format!("tab window {:?}", tab()));
    checks.check("3 the switch off stops the watcher", !core.watching.load(Ordering::SeqCst), format!("watching {}", core.watching.load(Ordering::SeqCst)));
    step("3 the switch for the tab, on");
    tab_set(app.clone(), true);
    wait(3200);
    checks.check("3 the switch on starts the watcher and the tab is back", core.watching.load(Ordering::SeqCst) && tab().is_some(), format!("watching {}, tab window {:?}", core.watching.load(Ordering::SeqCst), tab()));

    // 2, the rest: a double click on the grip sends the bar home.
    step("2 opening the bar again for the double click");
    post(CGEventType::MouseMoved, w * 0.5, h * 0.5);
    open(app);
    wait(1500);
    let Some(moved_bar) = app.get_webview_window(BAR).and_then(|b| window_rect(&b)) else {
        return checks.check("2 the bar opens again", false, "no bar window".into());
    };
    checks.check("2 the bar opens where it was dragged to", (moved_bar.x - after.x).abs() < 3.0 && (moved_bar.y - after.y).abs() < 3.0, format!("opened at {moved_bar:?}, was left at {after:?}"));
    step("2 the double click on the grip");
    let grip = grip_of(&moved_bar);
    post(CGEventType::MouseMoved, grip.0, grip.1);
    wait(300);
    for nth in [1, 2] {
        post_click(CGEventType::LeftMouseDown, grip.0, grip.1, nth);
        wait(50);
        post_click(CGEventType::LeftMouseUp, grip.0, grip.1, nth);
        wait(90);
    }
    wait(900);
    let home = app.get_webview_window(BAR).and_then(|b| window_rect(&b)).unwrap_or(moved_bar);
    let back = (home.x - before.x).abs() < 3.0 && (home.y - before.y).abs() < 3.0;
    checks.check("2 a double click on the grip sends the bar back", back, format!("bar at {home:?}, it opened by default at {before:?}"));
    checks.check("2 and its place is forgotten", bar_place(app, middle_of(&home)).is_none(), format!("barPlaces has {:?} for this display", bar_place(app, middle_of(&home))));
    step("2 closing the round at the end");
    close(app);
    wait(800);
}

fn wait(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

fn glide(kind: CGEventType, from: (f64, f64), to: (f64, f64)) {
    for i in 1..=20 {
        let t = i as f64 / 20.0;
        post(kind, from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
        wait(15);
    }
}

fn say(app: &AppHandle, step: &str) {
    let v = view(app);
    eprintln!("selftest {step}: picking={} tool={:?} picks={} noting={:?} recording={:?} trouble={:?}", v.picking, v.tool, v.picks.len(), v.noting, v.recording, v.trouble);
    for (i, p) in v.picks.iter().enumerate() {
        eprintln!("selftest   {}. {:?}", i + 1, serde_json::to_string(p).unwrap_or_default());
    }
}

pub fn run(app: AppHandle) {
    // The scan asks every app on screen a few thousand questions; the others are short.
    watchdog(Duration::from_secs(if std::env::var_os("CLIPFRAMES_SELFTEST_SCAN").is_some() { 300 } else { 60 }));
    thread::spawn(move || {
        walk(&app);
        DONE.store(true, Ordering::SeqCst);
    });
}

fn walk(app: &AppHandle) {
    {
        step("opening the bar");
        wait(1500);
        let bounds = CGDisplay::main().bounds();
        let (w, h) = (bounds.size.width, bounds.size.height);
        eprintln!("selftest display: {w} x {h}, in front: {:?}", element::foreground());
        open(&app);
        wait(1500);
        say(&app, "opened");
        let v = view(&app);
        if !v.picking || v.trouble.is_some() {
            eprintln!("selftest stopped: the round did not start, nothing was clicked");
            return;
        }

        // With CLIPFRAMES_SELFTEST_PLACEMENT set: the checks a hand would have to make.
        if std::env::var_os("CLIPFRAMES_SELFTEST_PLACEMENT").is_some() {
            placement(&app, w, h);
            eprintln!("selftest done");
            return;
        }

        // With CLIPFRAMES_SELFTEST_SCAN set: read a grid of points over the main display and
        // print each different thing found, then stop. Nothing is clicked.
        if std::env::var_os("CLIPFRAMES_SELFTEST_SCAN").is_some() {
            step("scanning the screen");
            let frame = element::Rect { x: 0.0, y: 0.0, width: w, height: h };
            let mut seen: Vec<String> = Vec::new();
            for row in 1..40 {
                for column in 1..56 {
                    let (x, y) = (frame.x + frame.width * column as f64 / 56.0, frame.y + frame.height * row as f64 / 40.0);
                    if let Ok(e) = element::element_at(x, y) {
                        let said = format!("{} {} [{}x{}]", e.headline(), e.selector(), e.frame.width as i32, e.frame.height as i32);
                        if !seen.contains(&said) {
                            eprintln!("selftest scan: {said}");
                            seen.push(said);
                        }
                    }
                }
            }
            close(&app);
            eprintln!("selftest done");
            return;
        }

        step("walk-through: hovering");
        // Hover: move across the screen and print what is read under the pointer.
        let middle = (w * 0.5, h * 0.42);
        glide(CGEventType::MouseMoved, (w * 0.3, h * 0.3), middle);
        wait(600);
        eprintln!("selftest under the pointer: {:?}", element::element_at(middle.0, middle.1).map(|e| (e.headline(), e.selector(), e.app, e.frame)));
        eprintln!("selftest shown: {:?}", app.state::<Core>().shown.lock().unwrap().as_ref().map(|_| "a highlight"));

        step("walk-through: picking an element");
        // Element: one click, one comment.
        post(CGEventType::LeftMouseDown, middle.0, middle.1);
        wait(60);
        post(CGEventType::LeftMouseUp, middle.0, middle.1);
        wait(1800);
        say(&app, "element picked");
        if view(&app).picks.is_empty() {
            eprintln!("selftest stopped: the click did not become a pick");
            close(&app);
            return;
        }
        note_set(app.clone(), 0, "selftest: an element".into(), None);
        hide_note(&app);
        wait(400);

        step("walk-through: a screenshot of an area");
        // Area: a drag becomes a screenshot.
        tool_set(app.clone(), Kind::Area);
        wait(300);
        let (a, b) = ((w * 0.30, h * 0.30), (w * 0.60, h * 0.55));
        post(CGEventType::MouseMoved, a.0, a.1);
        wait(100);
        post(CGEventType::LeftMouseDown, a.0, a.1);
        glide(CGEventType::LeftMouseDragged, a, b);
        post(CGEventType::LeftMouseUp, b.0, b.1);
        wait(2000);
        say(&app, "area picked");
        hide_note(&app);

        step("walk-through: recording a clip");
        // Clip: a drag starts recording; nothing is clicked while it runs.
        tool_set(app.clone(), Kind::Clip);
        wait(300);
        post(CGEventType::MouseMoved, a.0, a.1);
        wait(100);
        post(CGEventType::LeftMouseDown, a.0, a.1);
        glide(CGEventType::LeftMouseDragged, a, b);
        post(CGEventType::LeftMouseUp, b.0, b.1);
        wait(2600);
        say(&app, "recording");
        stop_recording(&app);
        wait(1500);
        say(&app, "clip picked");
        hide_note(&app);
        wait(300);

        step("walk-through: closing");
        let v = view(&app);
        eprintln!("selftest copied:\n{}", v.reference);
        if let Some((path, _)) = app.state::<Core>().folder.lock().unwrap().as_ref() {
            eprintln!("selftest folder: {}", path.display());
        }
        close(&app);
        wait(800);
        say(&app, "closed");
        eprintln!("selftest done");
    }
}
