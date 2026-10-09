//! macOS input for a picking round: an event tap on its own thread. Needs Accessibility.

use super::Input;
use core_foundation::base::TCFType;
use core_foundation::mach_port::CFMachPortRef;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, CallbackResult, EventField};
use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

const ESCAPE: i64 = 53;
const REFUSED: &str = "macOS refused the input tap. Clipframes needs the Accessibility permission.";

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
}

pub struct Source {
    run_loop: CFRunLoop,
    thread: Option<thread::JoinHandle<()>>,
}

// The run loop handle is only used to stop the loop, which is safe from any thread.
unsafe impl Send for Source {}

impl Source {
    /// `on_input` runs on the tap's thread and returns true to swallow the event.
    pub fn start(on_input: impl Fn(Input) -> bool + Send + 'static) -> Result<Source, String> {
        let (ready, started) = mpsc::channel::<Result<CFRunLoop, String>>();
        let thread = thread::Builder::new()
            .name("clipframes-input".into())
            .spawn(move || {
                let current = CFRunLoop::get_current();
                // The tap's own port, which the callback needs to switch the tap back on.
                let port = Arc::new(AtomicPtr::<c_void>::new(std::ptr::null_mut()));
                let own = port.clone();
                let tap = CGEventTap::new(
                    CGEventTapLocation::Session,
                    CGEventTapPlacement::HeadInsertEventTap,
                    CGEventTapOptions::Default,
                    vec![
                        CGEventType::MouseMoved,
                        CGEventType::LeftMouseDragged,
                        CGEventType::LeftMouseDown,
                        CGEventType::LeftMouseUp,
                        CGEventType::KeyDown,
                    ],
                    move |_proxy, kind, event| {
                        // The system switches a tap off when its callback was slow once (a busy
                        // machine, a debugger) and says so with one of these. Until the tap is
                        // switched on again the round looks open but hears nothing: clicks go
                        // to the app underneath and Esc is not caught. What arrives with the
                        // notice is not an event and must not be read.
                        if matches!(kind, CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput) {
                            let port = own.load(Ordering::SeqCst);
                            if !port.is_null() {
                                unsafe { CGEventTapEnable(port.cast(), true) };
                            }
                            return CallbackResult::Keep;
                        }
                        let p = event.location();
                        let input = match kind {
                            CGEventType::MouseMoved | CGEventType::LeftMouseDragged => Input::Move(p.x, p.y),
                            CGEventType::LeftMouseDown => Input::Down(p.x, p.y),
                            CGEventType::LeftMouseUp => Input::Up(p.x, p.y),
                            CGEventType::KeyDown if event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) == ESCAPE => Input::Cancel,
                            _ => return CallbackResult::Keep,
                        };
                        if on_input(input) { CallbackResult::Drop } else { CallbackResult::Keep }
                    },
                );
                let source = tap.as_ref().ok().and_then(|tap| tap.mach_port().create_runloop_source(0).ok());
                let (Ok(tap), Some(source)) = (tap, source) else {
                    let _ = ready.send(Err(REFUSED.into()));
                    return;
                };
                port.store(tap.mach_port().as_concrete_TypeRef().cast(), Ordering::SeqCst);
                current.add_source(&source, unsafe { kCFRunLoopCommonModes });
                tap.enable();
                let _ = ready.send(Ok(current.clone()));
                CFRunLoop::run_current();
                // The tap is taken out of the system when `tap` goes, here.
            })
            .map_err(|e| e.to_string())?;
        let run_loop = started.recv().map_err(|e| e.to_string())??;
        Ok(Source { run_loop, thread: Some(thread) })
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            stop(&self.run_loop, thread);
        }
    }
}

/// Stops a thread that runs `run_loop` and waits for it. Stopping a run loop that has not
/// started running yet does nothing, and the thread hands its loop over a moment before it
/// runs it: one stop sent in that moment would be lost, the wait would never end, and the tap
/// would stay on. So the stop is repeated until the thread has gone.
fn stop(run_loop: &CFRunLoop, thread: thread::JoinHandle<()>) {
    run_loop.stop();
    while !thread.is_finished() {
        thread::sleep(Duration::from_millis(2));
        run_loop.stop();
    }
    let _ = thread.join();
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_foundation::date::CFAbsoluteTimeGetCurrent;
    use core_foundation::runloop::{CFRunLoopTimer, CFRunLoopTimerRef};

    #[test]
    fn a_stop_that_comes_before_the_run_loop_runs_is_not_lost() {
        extern "C" fn nothing(_: CFRunLoopTimerRef, _: *mut c_void) {}
        let (ready, started) = mpsc::channel::<CFRunLoop>();
        let thread = thread::spawn(move || {
            let current = CFRunLoop::get_current();
            // Something to wait for, as the tap's source is in the app.
            let timer = CFRunLoopTimer::new(unsafe { CFAbsoluteTimeGetCurrent() } + 3600.0, 0.0, 0, 0, nothing, std::ptr::null_mut());
            current.add_timer(&timer, unsafe { kCFRunLoopCommonModes });
            let _ = ready.send(current.clone());
            thread::sleep(Duration::from_millis(100)); // the thread loses the processor here
            CFRunLoop::run_current();
        });
        let run_loop = started.recv().unwrap();
        // One stop now does nothing: the loop is not running yet.
        run_loop.stop();
        thread::sleep(Duration::from_millis(300));
        assert!(!thread.is_finished(), "a single early stop is lost");
        // Returns only once the thread has gone; before the fix this waited for ever.
        stop(&run_loop, thread);
    }
}
