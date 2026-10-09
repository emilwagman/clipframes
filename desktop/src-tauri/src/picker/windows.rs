//! Windows input for a picking round: low-level mouse and keyboard hooks on their own thread,
//! which runs a message loop for as long as the round lasts. No permission needed.

use super::Input;
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::mpsc;
use std::thread;

type Handle = *mut c_void;

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct MouseHook {
    pt: Point,
    mouse_data: u32,
    flags: u32,
    time: u32,
    extra: usize,
}

#[repr(C)]
struct KeyHook {
    vk: u32,
    scan: u32,
    flags: u32,
    time: u32,
    extra: usize,
}

#[repr(C)]
struct Msg {
    hwnd: Handle,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt: Point,
}

const WH_KEYBOARD_LL: i32 = 13;
const WH_MOUSE_LL: i32 = 14;
const WM_QUIT: u32 = 0x0012;
const WM_KEYDOWN: usize = 0x0100;
const WM_MOUSEMOVE: usize = 0x0200;
const WM_LBUTTONDOWN: usize = 0x0201;
const WM_LBUTTONUP: usize = 0x0202;
const VK_ESCAPE: u32 = 0x1B;

#[link(name = "user32")]
extern "system" {
    fn SetWindowsHookExW(id: i32, proc_: unsafe extern "system" fn(i32, usize, isize) -> isize, module: Handle, thread: u32) -> Handle;
    fn UnhookWindowsHookEx(hook: Handle) -> i32;
    fn CallNextHookEx(hook: Handle, code: i32, wparam: usize, lparam: isize) -> isize;
    fn GetMessageW(msg: *mut c_void, hwnd: Handle, min: u32, max: u32) -> i32;
    fn PostThreadMessageW(thread: u32, msg: u32, wparam: usize, lparam: isize) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentThreadId() -> u32;
}

thread_local! {
    /// The round's handler, on the hook thread only.
    static HANDLER: RefCell<Option<Box<dyn Fn(Input) -> bool>>> = const { RefCell::new(None) };
}

fn dispatch(input: Input) -> bool {
    HANDLER.with(|h| h.borrow().as_ref().map(|f| f(input)).unwrap_or(false))
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: usize, lparam: isize) -> isize {
    if code >= 0 {
        let m = &*(lparam as *const MouseHook);
        let (x, y) = (m.pt.x as f64, m.pt.y as f64);
        let input = match wparam {
            WM_MOUSEMOVE => Some(Input::Move(x, y)),
            WM_LBUTTONDOWN => Some(Input::Down(x, y)),
            WM_LBUTTONUP => Some(Input::Up(x, y)),
            _ => None,
        };
        if input.is_some_and(dispatch) {
            return 1; // swallowed
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

unsafe extern "system" fn key_proc(code: i32, wparam: usize, lparam: isize) -> isize {
    if code >= 0 && wparam == WM_KEYDOWN && (*(lparam as *const KeyHook)).vk == VK_ESCAPE && dispatch(Input::Cancel) {
        return 1;
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

pub struct Source {
    thread_id: u32,
    thread: Option<thread::JoinHandle<()>>,
}

impl Source {
    /// `on_input` runs on the hook thread and returns true to swallow the event.
    pub fn start(on_input: impl Fn(Input) -> bool + Send + 'static) -> Result<Source, String> {
        let (ready, started) = mpsc::channel::<Result<u32, String>>();
        let thread = thread::Builder::new()
            .name("clipframes-input".into())
            .spawn(move || unsafe {
                HANDLER.with(|h| *h.borrow_mut() = Some(Box::new(on_input)));
                let mouse = SetWindowsHookExW(WH_MOUSE_LL, mouse_proc, std::ptr::null_mut(), 0);
                let keys = SetWindowsHookExW(WH_KEYBOARD_LL, key_proc, std::ptr::null_mut(), 0);
                if mouse.is_null() {
                    let _ = ready.send(Err("Windows refused the mouse hook.".into()));
                    return;
                }
                let _ = ready.send(Ok(GetCurrentThreadId()));
                // Low-level hooks are called through this thread's message loop.
                let mut msg: Msg = std::mem::zeroed();
                while GetMessageW((&mut msg as *mut Msg).cast(), std::ptr::null_mut(), 0, 0) > 0 {}
                UnhookWindowsHookEx(mouse);
                if !keys.is_null() {
                    UnhookWindowsHookEx(keys);
                }
                HANDLER.with(|h| *h.borrow_mut() = None);
            })
            .map_err(|e| e.to_string())?;
        let thread_id = started.recv().map_err(|e| e.to_string())??;
        Ok(Source { thread_id, thread: Some(thread) })
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) };
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
