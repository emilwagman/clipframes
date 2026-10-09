//! A picking round: follow the pointer, say what is under it, and take the click.
//!
//! The system's input thread must never wait, so it only records the newest pointer position.
//! One worker thread reads the element there; positions it did not get to are dropped, because
//! only the latest matters. The input source (an event tap, a mouse hook) exists only while a
//! round is running.

use crate::element::{self, ElementInfo, Rect};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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

/// What the system's input thread reports. Coordinates are global screen points, top-left origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    Move(f64, f64),
    Down(f64, f64),
    Up(f64, f64),
    /// Esc.
    Cancel,
}

/// What the pointer does during a round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    /// Hover names elements, a click picks one.
    Element = 0,
    /// A drag marks out an area.
    Area = 1,
    /// Recording: clicks go to the app as usual and are only noted.
    Watch = 2,
}

impl Mode {
    fn from(n: u8) -> Mode {
        match n {
            1 => Mode::Area,
            2 => Mode::Watch,
            _ => Mode::Element,
        }
    }
}

/// The rectangle between two corners, whichever way it was dragged.
pub fn span(a: (f64, f64), b: (f64, f64)) -> Rect {
    Rect { x: a.0.min(b.0), y: a.1.min(b.1), width: (a.0 - b.0).abs(), height: (a.1 - b.1).abs() }
}

/// What a round tells the interface.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Event {
    /// The pointer is over this element now (`None`: nothing readable there).
    Hover { x: f64, y: f64, element: Option<ElementInfo>, read_ms: f64 },
    /// The user clicked this element.
    Pick { x: f64, y: f64, element: ElementInfo },
    /// An area is being dragged (`None`: the drag was dropped).
    Drag { rect: Option<Rect> },
    /// The user finished dragging this area.
    Area { rect: Rect },
    /// While watching: the user clicked here, and the click went to the app.
    Click { x: f64, y: f64, element: Option<ElementInfo> },
    Cancel,
}

#[derive(Default)]
struct Shared {
    /// The newest pointer position not read yet.
    pending: Mutex<Option<(f64, f64)>>,
    wake: Condvar,
    /// The last answer, so a click on an element just hovered needs no second reading.
    last: Mutex<Option<(f64, f64, ElementInfo)>>,
    /// Clipframes' own windows: clicks there are not picks.
    exempt: Mutex<Vec<Rect>>,
    mode: AtomicU8,
    /// Where the drag in progress began.
    drag: Mutex<Option<(f64, f64)>>,
    running: AtomicBool,
}

/// One picking round. Dropping it ends the round.
pub struct Picker {
    shared: Arc<Shared>,
    source: Option<platform::Source>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Picker {
    /// Starts a round. `emit` is called from background threads.
    pub fn start(emit: impl Fn(Event) + Send + Sync + 'static) -> Result<Picker, String> {
        let shared = Arc::new(Shared::default());
        shared.running.store(true, Ordering::SeqCst);
        let emit: Arc<dyn Fn(Event) + Send + Sync> = Arc::new(emit);

        let worker = {
            let (shared, emit) = (shared.clone(), emit.clone());
            thread::Builder::new().name("clipframes-reader".into()).spawn(move || read_loop(shared, emit)).map_err(|e| e.to_string())?
        };

        let source = {
            let (shared, emit) = (shared.clone(), emit.clone());
            platform::Source::start(move |input| handle(&shared, &emit, input))?
        };
        Ok(Picker { shared, source: Some(source), worker: Some(worker) })
    }

    /// Where Clipframes' own windows are, so clicks on the bar or the comment box pass through.
    pub fn set_exempt(&self, rects: Vec<Rect>) {
        *self.shared.exempt.lock().unwrap() = rects;
    }

    pub fn set_mode(&self, mode: Mode) {
        self.shared.mode.store(mode as u8, Ordering::SeqCst);
        *self.shared.drag.lock().unwrap() = None;
        *self.shared.pending.lock().unwrap() = None;
    }
}

impl Drop for Picker {
    fn drop(&mut self) {
        self.shared.running.store(false, Ordering::SeqCst);
        self.shared.wake.notify_all();
        self.source.take(); // stops the system input source
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

fn exempt(shared: &Shared, x: f64, y: f64) -> bool {
    shared.exempt.lock().unwrap().iter().any(|r| x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height)
}

/// Runs on the system's input thread: must return at once. Returns true to swallow the input.
fn handle(shared: &Shared, emit: &Arc<dyn Fn(Event) + Send + Sync>, input: Input) -> bool {
    let mode = Mode::from(shared.mode.load(Ordering::SeqCst));
    match input {
        Input::Move(x, y) => {
            // A drag keeps following the pointer even across Clipframes' own windows.
            let dragging = shared.drag.lock().unwrap().is_some();
            if mode != Mode::Watch && (dragging || !exempt(shared, x, y)) {
                *shared.pending.lock().unwrap() = Some((x, y));
                shared.wake.notify_one();
            }
            false
        }
        Input::Down(x, y) => {
            if exempt(shared, x, y) {
                return false;
            }
            match mode {
                // The press is swallowed so the app underneath never sees a click begin.
                Mode::Element => true,
                Mode::Area => {
                    *shared.drag.lock().unwrap() = Some((x, y));
                    true
                }
                Mode::Watch => {
                    let emit = emit.clone();
                    thread::spawn(move || emit(Event::Click { x, y, element: quick(x, y) }));
                    false
                }
            }
        }
        Input::Up(x, y) => {
            if let Some(start) = shared.drag.lock().unwrap().take() {
                let rect = span(start, (x, y));
                *shared.pending.lock().unwrap() = None;
                // A click without a drag is not an area.
                emit(if rect.width >= 8.0 && rect.height >= 8.0 { Event::Area { rect } } else { Event::Drag { rect: None } });
                return true;
            }
            if mode != Mode::Element || exempt(shared, x, y) {
                // In area mode a release with no drag belongs to a press that was let through.
                return mode == Mode::Area && !exempt(shared, x, y);
            }
            // The click asks once more, for the full description. The hover answer is the
            // fallback when the app does not reply.
            let known = shared.last.lock().unwrap().clone().filter(|(_, _, e)| contains(&e.frame, x, y)).map(|(_, _, e)| e);
            let emit = emit.clone();
            thread::spawn(move || {
                if let Some(element) = full(x, y).or(known) {
                    emit(Event::Pick { x, y, element });
                }
            });
            true
        }
        Input::Cancel => {
            *shared.drag.lock().unwrap() = None;
            emit(Event::Cancel);
            true
        }
    }
}

#[cfg(not(test))]
fn quick(x: f64, y: f64) -> Option<ElementInfo> {
    element::element_at(x, y).ok()
}

#[cfg(test)]
fn quick(_x: f64, _y: f64) -> Option<ElementInfo> {
    None
}

#[cfg(not(test))]
fn full(x: f64, y: f64) -> Option<ElementInfo> {
    element::element_full_at(x, y).ok()
}

/// Tests never read the real screen.
#[cfg(test)]
fn full(_x: f64, _y: f64) -> Option<ElementInfo> {
    None
}

fn contains(r: &Rect, x: f64, y: f64) -> bool {
    r.width > 0.0 && x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

fn read_loop(shared: Arc<Shared>, emit: Arc<dyn Fn(Event) + Send + Sync>) {
    let mut last_housekeeping = Instant::now();
    while shared.running.load(Ordering::SeqCst) {
        let next = {
            let mut pending = shared.pending.lock().unwrap();
            while pending.is_none() && shared.running.load(Ordering::SeqCst) {
                pending = shared.wake.wait_timeout(pending, Duration::from_millis(500)).unwrap().0;
            }
            pending.take()
        };
        let Some((x, y)) = next else { continue };
        if Mode::from(shared.mode.load(Ordering::SeqCst)) == Mode::Area {
            // No element reading while an area is drawn: only the rectangle so far.
            // Sent while holding the drag, so it can never arrive after the drag has ended.
            let drag = shared.drag.lock().unwrap();
            if let Some(start) = *drag {
                emit(Event::Drag { rect: Some(span(start, (x, y))) });
            }
            continue;
        }
        let started = Instant::now();
        let element = element::element_at(x, y).ok();
        let read_ms = started.elapsed().as_secs_f64() * 1000.0;
        *shared.last.lock().unwrap() = element.clone().map(|e| (x, y, e));
        emit(Event::Hover { x, y, element, read_ms });

        if last_housekeeping.elapsed() > Duration::from_secs(60) {
            element::sleep_idle(Duration::from_secs(300));
            last_housekeeping = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn round() -> (Arc<Shared>, Arc<dyn Fn(Event) + Send + Sync>, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let emit: Arc<dyn Fn(Event) + Send + Sync> = Arc::new(move |e| {
            let _ = tx.lock().unwrap().send(e);
        });
        (Arc::new(Shared::default()), emit, rx)
    }

    #[test]
    fn a_move_is_never_swallowed_and_only_the_newest_position_waits() {
        let (shared, emit, _rx) = round();
        assert!(!handle(&shared, &emit, Input::Move(10.0, 10.0)));
        assert!(!handle(&shared, &emit, Input::Move(20.0, 25.0)));
        assert_eq!(*shared.pending.lock().unwrap(), Some((20.0, 25.0)));
    }

    #[test]
    fn a_click_is_swallowed_so_the_app_underneath_never_sees_it() {
        let (shared, emit, _rx) = round();
        assert!(handle(&shared, &emit, Input::Down(300.0, 300.0)));
        assert!(handle(&shared, &emit, Input::Up(300.0, 300.0)));
    }

    #[test]
    fn clicks_and_moves_on_clipframes_own_windows_pass_through() {
        let (shared, emit, _rx) = round();
        *shared.exempt.lock().unwrap() = vec![Rect { x: 100.0, y: 800.0, width: 400.0, height: 60.0 }];
        assert!(!handle(&shared, &emit, Input::Down(150.0, 820.0)));
        assert!(!handle(&shared, &emit, Input::Up(150.0, 820.0)));
        handle(&shared, &emit, Input::Move(150.0, 820.0));
        assert_eq!(*shared.pending.lock().unwrap(), None, "hovering our own bar must not ask for an element");
    }

    #[test]
    fn a_click_falls_back_to_the_hover_answer_when_the_app_does_not_reply() {
        let (shared, emit, rx) = round();
        let button = ElementInfo { role: "Button".into(), name: "New invoice".into(), frame: Rect { x: 100.0, y: 40.0, width: 110.0, height: 39.0 }, ..Default::default() };
        *shared.last.lock().unwrap() = Some((120.0, 50.0, button));
        assert!(handle(&shared, &emit, Input::Up(180.0, 70.0)));
        match rx.recv_timeout(Duration::from_secs(2)).expect("a pick") {
            Event::Pick { element, .. } => assert_eq!(element.headline(), "Button \"New invoice\""),
            other => panic!("expected a pick, got {other:?}"),
        }
    }

    #[test]
    fn a_drag_in_area_mode_becomes_an_area_and_nothing_reaches_the_app() {
        let (shared, emit, rx) = round();
        shared.mode.store(Mode::Area as u8, Ordering::SeqCst);
        assert!(handle(&shared, &emit, Input::Down(400.0, 300.0)));
        assert!(!handle(&shared, &emit, Input::Move(250.0, 380.0)), "moves always pass");
        assert!(handle(&shared, &emit, Input::Up(100.0, 500.0)));
        match rx.recv_timeout(Duration::from_secs(1)).expect("an area") {
            Event::Area { rect } => assert_eq!(rect, Rect { x: 100.0, y: 300.0, width: 300.0, height: 200.0 }),
            other => panic!("expected an area, got {other:?}"),
        }
    }

    #[test]
    fn a_click_without_a_drag_in_area_mode_is_dropped() {
        let (shared, emit, rx) = round();
        shared.mode.store(Mode::Area as u8, Ordering::SeqCst);
        assert!(handle(&shared, &emit, Input::Down(400.0, 300.0)));
        assert!(handle(&shared, &emit, Input::Up(402.0, 301.0)));
        assert!(matches!(rx.recv_timeout(Duration::from_secs(1)), Ok(Event::Drag { rect: None })));
    }

    #[test]
    fn while_recording_clicks_reach_the_app_and_are_noted() {
        let (shared, emit, rx) = round();
        shared.mode.store(Mode::Watch as u8, Ordering::SeqCst);
        assert!(!handle(&shared, &emit, Input::Down(50.0, 60.0)));
        assert!(!handle(&shared, &emit, Input::Up(50.0, 60.0)));
        assert!(matches!(rx.recv_timeout(Duration::from_secs(1)), Ok(Event::Click { .. })));
    }

    #[test]
    fn esc_cancels_and_is_swallowed() {
        let (shared, emit, rx) = round();
        assert!(handle(&shared, &emit, Input::Cancel));
        assert!(matches!(rx.recv_timeout(Duration::from_secs(1)), Ok(Event::Cancel)));
    }
}
