//! macOS input for a picking round: an event tap on its own thread. Needs Accessibility.

use super::Input;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, CallbackResult, EventField};
use std::sync::mpsc;
use std::thread;

const ESCAPE: i64 = 53;

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
                let ready_ok = ready.clone();
                let tap = CGEventTap::with_enabled(
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
                    || {
                        let _ = ready_ok.send(Ok(current.clone()));
                        CFRunLoop::run_current();
                    },
                );
                if tap.is_err() {
                    let _ = ready.send(Err("macOS refused the input tap. Clipframes needs the Accessibility permission.".into()));
                }
            })
            .map_err(|e| e.to_string())?;
        let run_loop = started.recv().map_err(|e| e.to_string())??;
        // Touch the mode constant so the linker keeps CoreFoundation's run loop symbols.
        let _ = unsafe { kCFRunLoopCommonModes };
        Ok(Source { run_loop, thread: Some(thread) })
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        self.run_loop.stop();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
