//! A picking round: follow the pointer, say what is under it, and take the click.
//!
//! The system's input thread must never wait, so it only records the newest pointer position.
//! One worker thread reads the element there; positions it did not get to are dropped, because
//! only the latest matters. The input source (an event tap, a mouse hook) exists only while a
//! round is running.

use crate::element::{self, ElementInfo, Rect};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
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
    /// The user clicked this element. `token` is what the pick is known by for `Located`;
    /// `read_ms` is how long ago the click was: the time reading the element took.
    Pick { x: f64, y: f64, element: ElementInfo, token: u64, read_ms: f64 },
    /// Sent after `Pick`, once the look through the page or window has been taken: which one
    /// of several that read the same the element is, and the heading it is under. Both may
    /// be nothing. `took_ms` is how long the look took.
    Located { token: u64, occurrence: Option<(u32, u32)>, heading: Option<element::Heading>, took_ms: f64 },
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
    /// The last press was on one of Clipframes' own windows and went to it.
    own_press: AtomicBool,
    /// The bar is being moved by its grip: nothing it passes over is read or outlined.
    held: AtomicBool,
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
            platform::Source::start(move |input| handle(&shared, &emit, input))
        };
        match source {
            Ok(source) => Ok(Picker { shared, source: Some(source), worker: Some(worker) }),
            Err(message) => {
                // The reader was already started: without this it would wake twice a second
                // for the rest of the run, one more for every failed open.
                shared.running.store(false, Ordering::SeqCst);
                shared.wake.notify_all();
                let _ = worker.join();
                Err(message)
            }
        }
    }

    /// Where Clipframes' own windows are, so clicks on the bar or the comment box pass through.
    pub fn set_exempt(&self, rects: Vec<Rect>) {
        *self.shared.exempt.lock().unwrap() = rects;
    }

    /// Said when the bar's grip is pressed and while the bar moves, and taken back when the
    /// bar has come to rest. A release of the button takes it back too, but nothing depends
    /// on the release being seen.
    pub fn hold(&self, on: bool) {
        self.shared.held.store(on, Ordering::SeqCst);
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
    // An input source that outlives its round must be harmless: nothing is swallowed.
    if !shared.running.load(Ordering::SeqCst) {
        return false;
    }
    let mode = Mode::from(shared.mode.load(Ordering::SeqCst));
    match input {
        Input::Move(x, y) => {
            // A drag keeps following the pointer even across Clipframes' own windows.
            let dragging = shared.drag.lock().unwrap().is_some();
            let held = shared.held.load(Ordering::SeqCst);
            if mode != Mode::Watch && !held && (dragging || !exempt(shared, x, y)) {
                *shared.pending.lock().unwrap() = Some((x, y));
                shared.wake.notify_one();
            }
            false
        }
        Input::Down(x, y) => {
            let own = exempt(shared, x, y);
            shared.own_press.store(own, Ordering::SeqCst);
            if own {
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
            shared.held.store(false, Ordering::SeqCst);
            // A press the bar got is the bar's to the end: dragged off and let go elsewhere,
            // it is not a pick, and the bar must see the button come up.
            if shared.own_press.swap(false, Ordering::SeqCst) {
                return false;
            }
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
            let last = shared.last.lock().unwrap().clone();
            // Whether the pointer was read where it is now: not when it jumped here, or moved
            // faster than the reader, or its moves were not followed.
            let hovered = last.as_ref().is_some_and(|(lx, ly, _)| (lx - x).abs() <= 2.0 && (ly - y).abs() <= 2.0);
            let known = last.filter(|(_, _, e)| contains(&e.frame, x, y)).map(|(_, _, e)| e);
            let emit = emit.clone();
            thread::spawn(move || {
                static TOKENS: AtomicU64 = AtomicU64::new(0);
                let clicked = Instant::now();
                // A point that was never hovered is asked about once before the reading that
                // counts. An app may answer the first question about a point with what is
                // around it (a browser on Windows: the whole page) and only the next with the
                // element; a click after a hover has always been that next one. Its answer
                // is also a nearer fallback than an older hover somewhere else in the frame.
                let first = if hovered { None } else { quick(x, y) };
                let (read, mut more) = match full(x, y) {
                    Some(picked) => (Some(picked.info.clone()), Some(picked)),
                    None => (None, None),
                };
                let Some(element) = read.or(first).or(known) else { return };
                let token = TOKENS.fetch_add(1, Ordering::SeqCst) + 1;
                emit(Event::Pick { x, y, element, token, read_ms: clicked.elapsed().as_secs_f64() * 1000.0 });
                // The pick is shown and copied by now: `emit` returns when that is done. Only
                // then the slower look, on this thread, which is neither the input thread nor
                // the interface's.
                if let Some(picked) = more.as_mut().filter(|picked| picked.has_more()) {
                    let looking = Instant::now();
                    let located = picked.whereabouts().unwrap_or_default();
                    emit(Event::Located { token, occurrence: located.occurrence, heading: located.heading, took_ms: looking.elapsed().as_secs_f64() * 1000.0 });
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

/// Tests never read the real screen: they only note where a reading was asked for.
#[cfg(test)]
fn quick(x: f64, y: f64) -> Option<ElementInfo> {
    tests::QUICK.lock().unwrap().push((x, y));
    (x == tests::SLOW_TO_ANSWER).then(|| ElementInfo { role: "Button".into(), name: "New invoice".into(), ..Default::default() })
}

#[cfg(not(test))]
fn full(x: f64, y: f64) -> Option<element::Picked> {
    element::element_picked_at(x, y).ok()
}

/// Tests never read the real screen. One point stands for an element with a second look to
/// take, which says so loudly if it is taken before the pick has been handed over.
#[cfg(test)]
fn full(x: f64, _y: f64) -> Option<element::Picked> {
    (x == tests::TWICE).then(|| element::Picked::for_test(ElementInfo { role: "Text".into(), name: "Paid".into(), ..Default::default() }, || {
        thread::sleep(Duration::from_millis(60));
        Some(element::locate::Located { occurrence: Some((2, 4)), heading: None })
    }))
}

fn contains(r: &Rect, x: f64, y: f64) -> bool {
    r.width > 0.0 && x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

fn read_loop(shared: Arc<Shared>, emit: Arc<dyn Fn(Event) + Send + Sync>) {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// Where the element with a second look is (see `full`).
    pub const TWICE: f64 = 777.0;
    /// Where the first, quick reading answers and the full one does not.
    pub const SLOW_TO_ANSWER: f64 = 555.0;
    /// Every point `quick` was asked about. Tests share it, so each looks for its own point.
    pub static QUICK: Mutex<Vec<(f64, f64)>> = Mutex::new(Vec::new());

    fn asked(x: f64, y: f64) -> usize {
        QUICK.lock().unwrap().iter().filter(|p| **p == (x, y)).count()
    }

    #[test]
    fn the_pick_is_handed_over_before_the_look_through_the_page_begins() {
        let (shared, emit, rx) = round();
        let clicked = Instant::now();
        assert!(handle(&shared, &emit, Input::Up(TWICE, 300.0)));
        let token = match rx.recv_timeout(Duration::from_secs(1)).expect("a pick") {
            Event::Pick { element, token, .. } => {
                assert_eq!((element.name.as_str(), element.occurrence), ("Paid", None), "as it was read, without the later facts");
                token
            }
            other => panic!("expected a pick, got {other:?}"),
        };
        assert!(clicked.elapsed() < Duration::from_millis(50), "the pick did not wait for the look: {:?}", clicked.elapsed());
        match rx.recv_timeout(Duration::from_secs(1)).expect("the later facts") {
            Event::Located { token: of, occurrence, took_ms, .. } => {
                assert_eq!((of, occurrence), (token, Some((2, 4))), "for the same pick");
                assert!(took_ms >= 60.0);
            }
            other => panic!("expected the later facts, got {other:?}"),
        }
    }

    fn round() -> (Arc<Shared>, Arc<dyn Fn(Event) + Send + Sync>, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let emit: Arc<dyn Fn(Event) + Send + Sync> = Arc::new(move |e| {
            let _ = tx.lock().unwrap().send(e);
        });
        let shared = Arc::new(Shared::default());
        shared.running.store(true, Ordering::SeqCst);
        (shared, emit, rx)
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
    fn a_press_on_the_bar_released_outside_it_is_not_a_pick() {
        let (shared, emit, rx) = round();
        *shared.exempt.lock().unwrap() = vec![Rect { x: 100.0, y: 800.0, width: 400.0, height: 60.0 }];
        let under = ElementInfo { role: "Button".into(), frame: Rect { x: 250.0, y: 250.0, width: 100.0, height: 100.0 }, ..Default::default() };
        *shared.last.lock().unwrap() = Some((300.0, 300.0, under));
        assert!(!handle(&shared, &emit, Input::Down(150.0, 820.0)), "the bar gets the press");
        assert!(!handle(&shared, &emit, Input::Up(300.0, 300.0)), "and the release that belongs to it");
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err(), "nothing the user never clicked is picked");
        // The next click is an ordinary one again.
        assert!(handle(&shared, &emit, Input::Down(300.0, 300.0)));
        assert!(handle(&shared, &emit, Input::Up(300.0, 300.0)));
        assert!(matches!(rx.recv_timeout(Duration::from_secs(1)), Ok(Event::Pick { .. })));
    }

    #[test]
    fn while_the_bar_is_dragged_nothing_under_the_pointer_is_read() {
        let (shared, emit, _rx) = round();
        *shared.exempt.lock().unwrap() = vec![Rect { x: 100.0, y: 800.0, width: 400.0, height: 60.0 }];
        assert!(!handle(&shared, &emit, Input::Down(110.0, 820.0)), "the grip gets the press");
        shared.held.store(true, Ordering::SeqCst); // what `Picker::hold` sets when the bar says its grip was pressed
        // The bar follows the pointer; its old place is all the picker knows until it rests.
        handle(&shared, &emit, Input::Move(300.0, 300.0));
        assert_eq!(*shared.pending.lock().unwrap(), None, "nothing is outlined on the way");
        assert!(!handle(&shared, &emit, Input::Up(300.0, 300.0)), "and letting go is not a pick");
        handle(&shared, &emit, Input::Move(310.0, 300.0));
        assert_eq!(*shared.pending.lock().unwrap(), Some((310.0, 300.0)), "afterwards pointing works as before");
    }

    #[test]
    fn a_bar_drag_whose_release_was_never_seen_ends_when_the_bar_rests() {
        let (shared, emit, rx) = round();
        *shared.exempt.lock().unwrap() = vec![Rect { x: 100.0, y: 800.0, width: 400.0, height: 60.0 }];
        assert!(!handle(&shared, &emit, Input::Down(110.0, 820.0)));
        shared.held.store(true, Ordering::SeqCst);
        handle(&shared, &emit, Input::Move(300.0, 300.0));
        // No release arrives. The bar comes to rest and says so (`Picker::hold(false)`).
        shared.held.store(false, Ordering::SeqCst);
        handle(&shared, &emit, Input::Move(640.0, 200.0));
        assert_eq!(*shared.pending.lock().unwrap(), Some((640.0, 200.0)), "the next move is followed");
        // And the next click is a pick, though the press before it never ended.
        assert!(handle(&shared, &emit, Input::Down(TWICE, 200.0)));
        assert!(handle(&shared, &emit, Input::Up(TWICE, 200.0)));
        assert!(matches!(rx.recv_timeout(Duration::from_secs(1)), Ok(Event::Pick { .. })));
    }

    #[test]
    fn a_click_where_the_pointer_was_never_read_asks_once_before_the_reading_that_counts() {
        let (shared, emit, rx) = round();
        // The last hover was somewhere else, on the page as a whole. The pointer then jumped.
        let page = ElementInfo { role: "Group".into(), frame: Rect { x: 0.0, y: 0.0, width: 1971.0, height: 1942.0 }, ..Default::default() };
        *shared.last.lock().unwrap() = Some((40.0, 900.0, page.clone()));
        assert!(handle(&shared, &emit, Input::Up(TWICE, 411.0)));
        match rx.recv_timeout(Duration::from_secs(1)).expect("a pick") {
            Event::Pick { element, .. } => assert_eq!(element.name, "Paid", "what is at the click, not the page hovered before"),
            other => panic!("expected a pick, got {other:?}"),
        }
        assert_eq!(asked(TWICE, 411.0), 1, "the point was asked about first");

        // Hovered where it is clicked: the hover was that first question.
        let (shared, emit, rx) = round();
        *shared.last.lock().unwrap() = Some((TWICE, 412.0, page.clone()));
        assert!(handle(&shared, &emit, Input::Up(TWICE, 413.0)));
        assert!(matches!(rx.recv_timeout(Duration::from_secs(1)), Ok(Event::Pick { .. })));
        assert_eq!(asked(TWICE, 413.0), 0);

        // The full reading gets no answer: the first one is used before an old hover.
        let (shared, emit, rx) = round();
        *shared.last.lock().unwrap() = Some((40.0, 900.0, page));
        assert!(handle(&shared, &emit, Input::Up(SLOW_TO_ANSWER, 414.0)));
        match rx.recv_timeout(Duration::from_secs(1)).expect("a pick") {
            Event::Pick { element, .. } => assert_eq!(element.headline(), "Button \"New invoice\""),
            other => panic!("expected a pick, got {other:?}"),
        }
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

    #[test]
    fn a_round_that_has_stopped_swallows_nothing_even_if_its_input_source_lives_on() {
        let (shared, emit, rx) = round();
        shared.running.store(false, Ordering::SeqCst); // what Picker::drop sets first
        assert!(!handle(&shared, &emit, Input::Down(300.0, 300.0)));
        assert!(!handle(&shared, &emit, Input::Up(300.0, 300.0)));
        assert!(!handle(&shared, &emit, Input::Cancel));
        assert!(rx.recv_timeout(Duration::from_millis(200)).is_err(), "and nothing is picked");
    }
}
