//! A walk through every tool with real pointer events, for a Mac nobody is sitting at.
//!
//! Built only with `--features selftest` and started with `--selftest`; it is never part of a
//! release. The app posts the pointer events itself (it holds the Accessibility permission),
//! so they travel the same road as a hand on the mouse: through the system, into the picker.
//! Every step prints what the round looks like to stderr.

use super::*;
use core_graphics::display::CGDisplay;
use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

fn post(kind: CGEventType, x: f64, y: f64) {
    let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else { return };
    if let Ok(event) = CGEvent::new_mouse_event(source, kind, CGPoint::new(x, y), CGMouseButton::Left) {
        event.post(CGEventTapLocation::HID);
    }
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
    thread::spawn(move || {
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

        // With CLIPFRAMES_SELFTEST_SCAN set: read a grid of points over the main display and
        // print each different thing found, then stop. Nothing is clicked.
        if std::env::var_os("CLIPFRAMES_SELFTEST_SCAN").is_some() {
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

        // Hover: move across the screen and print what is read under the pointer.
        let middle = (w * 0.5, h * 0.42);
        glide(CGEventType::MouseMoved, (w * 0.3, h * 0.3), middle);
        wait(600);
        eprintln!("selftest under the pointer: {:?}", element::element_at(middle.0, middle.1).map(|e| (e.headline(), e.selector(), e.app, e.frame)));
        eprintln!("selftest shown: {:?}", app.state::<Core>().shown.lock().unwrap().as_ref().map(|_| "a highlight"));

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

        let v = view(&app);
        eprintln!("selftest copied:\n{}", v.reference);
        if let Some((path, _)) = app.state::<Core>().folder.lock().unwrap().as_ref() {
            eprintln!("selftest folder: {}", path.display());
        }
        close(&app);
        wait(800);
        say(&app, "closed");
        eprintln!("selftest done");
    });
}
