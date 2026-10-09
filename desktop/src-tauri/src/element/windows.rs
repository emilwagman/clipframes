//! Windows: UI Automation. No permission needed. Chromium and Electron expose a page's
//! elements here too: AutomationId carries the DOM id and ClassName the class list.

use super::{ElementInfo, ReadError, Rect};
use uiautomation::patterns::UIValuePattern;
use uiautomation::types::Point;
use uiautomation::{UIAutomation, UIElement};

pub fn permitted() -> bool {
    true
}

pub fn pointer() -> Option<(f64, f64)> {
    #[repr(C)]
    struct P {
        x: i32,
        y: i32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn GetCursorPos(p: *mut P) -> i32;
    }
    let mut p = P { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut p) } != 0 { Some((p.x as f64, p.y as f64)) } else { None }
}

thread_local! {
    /// One connection per thread: making it is the slow part, and only the reader thread asks.
    static AUTOMATION: Option<UIAutomation> = UIAutomation::new().ok();
}

pub fn element_at(x: f64, y: f64) -> Result<ElementInfo, ReadError> {
    AUTOMATION.with(|automation| {
        let automation = automation.as_ref().ok_or(ReadError::Other("UI Automation is not available.".into()))?;
        let el = automation.element_from_point(Point::new(x as i32, y as i32)).map_err(|_| ReadError::Nothing)?;
        Ok(describe(&el, x as i32, y as i32))
    })
}

/// What the hover reading leaves out because each answer is a call into the other app: the
/// selector of a bare piece of text, what the element sits inside, and the page's address.
pub fn element_full_at(x: f64, y: f64) -> Result<ElementInfo, ReadError> {
    AUTOMATION.with(|automation| {
        let automation = automation.as_ref().ok_or(ReadError::Other("UI Automation is not available.".into()))?;
        let el = automation.element_from_point(Point::new(x as i32, y as i32)).map_err(|_| ReadError::Nothing)?;
        let mut info = describe(&el, x as i32, y as i32);
        let Ok(walker) = automation.get_control_view_walker() else { return Ok(info) };

        let mut parent = walker.get_parent(&el).ok();
        let mut nearest = true;
        let mut steps = 0;
        while let Some(p) = parent {
            let role = role_of(&p);
            if role == "Window" || steps > 40 {
                break;
            }
            if nearest && is_web(&p) && info.dom_id.is_empty() && info.dom_classes.is_empty() && info.role == "Text" {
                // A piece of text has no id or class of its own; the element holding it does.
                info.dom_id = p.get_automation_id().unwrap_or_default();
                info.dom_classes = p.get_classname().unwrap_or_default();
            }
            nearest = false;
            if role == "Document" {
                if info.url.is_empty() && is_web(&p) {
                    info.url = p.get_pattern::<UIValuePattern>().and_then(|v| v.get_value()).unwrap_or_default();
                }
            } else if role != "Pane" && info.path.len() < 4 {
                let name = p.get_name().unwrap_or_default();
                if !name.is_empty() && name.chars().count() <= 40 && name != info.name {
                    info.path.insert(0, name);
                }
            }
            steps += 1;
            parent = walker.get_parent(&p).ok();
        }
        Ok(info)
    })
}

fn is_web(el: &UIElement) -> bool {
    let framework = el.get_framework_id().unwrap_or_default();
    framework.eq_ignore_ascii_case("Chrome") || framework.eq_ignore_ascii_case("Gecko")
}

fn describe(el: &UIElement, x: i32, y: i32) -> ElementInfo {
    let class = el.get_classname().unwrap_or_default();
    let automation_id = el.get_automation_id().unwrap_or_default();
    let framework = el.get_framework_id().unwrap_or_default();
    let web = framework.eq_ignore_ascii_case("Chrome") || framework.eq_ignore_ascii_case("Gecko");

    let mut info = ElementInfo {
        role: role_of(el),
        name: el.get_name().unwrap_or_default(),
        pid: el.get_process_id().unwrap_or_default() as i32,
        ..Default::default()
    };
    if web {
        info.dom_id = automation_id;
        info.dom_classes = class;
    } else {
        info.identifier = automation_id;
    }
    if let Ok(r) = el.get_bounding_rectangle() {
        info.frame = Rect { x: r.get_left() as f64, y: r.get_top() as f64, width: r.get_width() as f64, height: r.get_height() as f64 };
    }
    // The window and the app come from the window system directly: asking UI Automation for
    // each parent is a call into the other app every step.
    let (title, pid) = win::top_window_at(x, y);
    info.app = win::app_name(if pid != 0 { pid } else { info.pid as u32 });
    // "Invoices - Google Chrome" says the app twice once the app is named.
    info.window = match title.strip_suffix(&info.app).map(|t| t.trim_end_matches([' ', '-', '\u{2013}', '\u{2014}'])) {
        Some(short) if !info.app.is_empty() && !short.is_empty() => short.to_string(),
        _ => title,
    };
    info
}

/// "ButtonControl" → "Button", to read like the macOS roles.
fn role_of(el: &UIElement) -> String {
    el.get_control_type().map(|t| format!("{:?}", t).trim_end_matches("Control").to_string()).unwrap_or_default()
}

pub fn sleep_idle(_older_than: std::time::Duration) {}

/// Asks the browser under a point for its full description of the page.
///
/// Chrome gives UI Automation a thin tree until something that looks like a screen reader
/// turns up: ids but no classes. Screen readers announce themselves by asking the page for
/// the IAccessible2 family of interfaces, so this asks for them once per window.
pub fn wake_at(x: f64, y: f64) -> bool {
    win::wake_at(x as i32, y as i32)
}

/// The few window-system calls this needs, declared by hand to keep the build small.
mod win {
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::sync::Mutex;

    type Handle = *mut c_void;

    #[repr(C)]
    struct P {
        x: i32,
        y: i32,
    }

    const GA_ROOT: u32 = 2;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    #[link(name = "user32")]
    extern "system" {
        fn WindowFromPoint(p: P) -> Handle;
        fn GetAncestor(hwnd: Handle, flags: u32) -> Handle;
        fn GetWindowTextW(hwnd: Handle, text: *mut u16, max: i32) -> i32;
        fn GetWindowThreadProcessId(hwnd: Handle, pid: *mut u32) -> u32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
        fn CloseHandle(handle: Handle) -> i32;
        fn QueryFullProcessImageNameW(process: Handle, flags: u32, name: *mut u16, size: *mut u32) -> i32;
    }

    #[link(name = "version")]
    extern "system" {
        fn GetFileVersionInfoSizeW(file: *const u16, handle: *mut u32) -> u32;
        fn GetFileVersionInfoW(file: *const u16, handle: u32, len: u32, data: *mut c_void) -> i32;
        fn VerQueryValueW(block: *const c_void, sub: *const u16, out: *mut *mut c_void, len: *mut u32) -> i32;
    }

    #[repr(C)]
    #[derive(Clone, Copy, PartialEq)]
    struct Guid(u32, u16, u16, [u8; 8]);

    const IID_IACCESSIBLE: Guid = Guid(0x618736e0, 0x3c3d, 0x11cf, [0x81, 0x0c, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);
    const IID_ISERVICE_PROVIDER: Guid = Guid(0x6d5140c1, 0x7436, 0x11ce, [0x80, 0x34, 0x00, 0xaa, 0x00, 0x60, 0x09, 0xfa]);
    /// What a screen reader asks a page for: IAccessible2, IAccessibleText, ISimpleDOMNode.
    const WAKING: [Guid; 3] = [
        Guid(0xe89f726e, 0xc4f4, 0x4c19, [0xbb, 0x19, 0xb6, 0x47, 0xd7, 0xfa, 0x84, 0x78]),
        Guid(0x24fd2ffb, 0x3aad, 0x4a08, [0x83, 0x35, 0xa3, 0xad, 0x89, 0xc0, 0xfb, 0x4b]),
        Guid(0x1814ceeb, 0x49e2, 0x407f, [0xaf, 0x99, 0xfa, 0x75, 0x5a, 0x7d, 0x26, 0x07]),
    ];
    const OBJID_CLIENT: u32 = 0xFFFF_FFFC;

    #[link(name = "oleacc")]
    extern "system" {
        fn AccessibleObjectFromWindow(hwnd: Handle, object: u32, iid: *const Guid, out: *mut *mut c_void) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn FindWindowExW(parent: Handle, after: Handle, class: *const u16, title: *const u16) -> Handle;
        fn GetClassNameW(hwnd: Handle, text: *mut u16, max: i32) -> i32;
    }

    /// The first three entries of every COM interface, and the fourth of IServiceProvider.
    #[repr(C)]
    struct Vtable {
        query_interface: unsafe extern "system" fn(*mut c_void, *const Guid, *mut *mut c_void) -> i32,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        query_service: unsafe extern "system" fn(*mut c_void, *const Guid, *const Guid, *mut *mut c_void) -> i32,
    }

    unsafe fn vtable(object: *mut c_void) -> &'static Vtable {
        &**(object as *mut *const Vtable)
    }

    /// True if a browser's page was found there and asked.
    pub fn wake_at(x: i32, y: i32) -> bool {
        static WOKEN: Mutex<Vec<usize>> = Mutex::new(Vec::new());
        unsafe {
            let top = GetAncestor(WindowFromPoint(P { x, y }), GA_ROOT);
            if top.is_null() {
                return false;
            }
            let mut class = [0u16; 64];
            let n = GetClassNameW(top, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
            // Chrome, Edge, Brave and the rest of the family share this window class.
            if !String::from_utf16_lossy(&class[..n]).starts_with("Chrome_WidgetWin") {
                return false;
            }
            let page = FindWindowExW(top, std::ptr::null_mut(), wide("Chrome_RenderWidgetHostHWND").as_ptr(), std::ptr::null());
            if page.is_null() {
                return false;
            }
            {
                let mut woken = WOKEN.lock().unwrap();
                if woken.contains(&(page as usize)) {
                    return true;
                }
                if woken.len() > 64 {
                    woken.clear();
                }
                woken.push(page as usize);
            }
            let mut accessible = std::ptr::null_mut();
            if AccessibleObjectFromWindow(page, OBJID_CLIENT, &IID_IACCESSIBLE, &mut accessible) < 0 || accessible.is_null() {
                return false;
            }
            let mut provider = std::ptr::null_mut();
            if (vtable(accessible).query_interface)(accessible, &IID_ISERVICE_PROVIDER, &mut provider) >= 0 && !provider.is_null() {
                for iid in &WAKING {
                    let mut out = std::ptr::null_mut();
                    if (vtable(provider).query_service)(provider, iid, iid, &mut out) >= 0 && !out.is_null() {
                        (vtable(out).release)(out);
                    }
                }
                (vtable(provider).release)(provider);
            }
            (vtable(accessible).release)(accessible);
            true
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// The title and process of the top-level window at a point.
    pub fn top_window_at(x: i32, y: i32) -> (String, u32) {
        unsafe {
            let hwnd = GetAncestor(WindowFromPoint(P { x, y }), GA_ROOT);
            if hwnd.is_null() {
                return (String::new(), 0);
            }
            let mut text = [0u16; 512];
            let n = GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32).max(0) as usize;
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            (String::from_utf16_lossy(&text[..n]), pid)
        }
    }

    /// "Google Chrome" for chrome.exe: the program's own description, or its file name.
    pub fn app_name(pid: u32) -> String {
        static NAMES: Mutex<Option<HashMap<u32, String>>> = Mutex::new(None);
        if pid == 0 {
            return String::new();
        }
        let mut names = NAMES.lock().unwrap();
        let names = names.get_or_insert_with(HashMap::new);
        if let Some(name) = names.get(&pid) {
            return name.clone();
        }
        let name = image_path(pid).map(|path| description(&path).unwrap_or_else(|| stem(&path))).unwrap_or_default();
        if names.len() > 256 {
            names.clear(); // process ids are reused; a small cache is enough
        }
        names.insert(pid, name.clone());
        name
    }

    fn image_path(pid: u32) -> Option<String> {
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if process.is_null() {
                return None;
            }
            let mut buffer = [0u16; 1024];
            let mut size = buffer.len() as u32;
            let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut size);
            CloseHandle(process);
            (ok != 0).then(|| String::from_utf16_lossy(&buffer[..size as usize]))
        }
    }

    fn stem(path: &str) -> String {
        let file = path.rsplit(['\\', '/']).next().unwrap_or(path);
        file.strip_suffix(".exe").or(file.strip_suffix(".EXE")).unwrap_or(file).to_string()
    }

    /// FileDescription from the program's version resource.
    fn description(path: &str) -> Option<String> {
        unsafe {
            let file = wide(path);
            let mut ignored = 0u32;
            let len = GetFileVersionInfoSizeW(file.as_ptr(), &mut ignored);
            if len == 0 {
                return None;
            }
            let mut data = vec![0u8; len as usize];
            if GetFileVersionInfoW(file.as_ptr(), 0, len, data.as_mut_ptr().cast()) == 0 {
                return None;
            }
            let (mut value, mut size) = (std::ptr::null_mut::<c_void>(), 0u32);
            if VerQueryValueW(data.as_ptr().cast(), wide("\\VarFileInfo\\Translation").as_ptr(), &mut value, &mut size) == 0 || size < 4 {
                return None;
            }
            let pair = std::slice::from_raw_parts(value as *const u16, 2);
            let key = wide(&format!("\\StringFileInfo\\{:04x}{:04x}\\FileDescription", pair[0], pair[1]));
            if VerQueryValueW(data.as_ptr().cast(), key.as_ptr(), &mut value, &mut size) == 0 || size == 0 {
                return None;
            }
            let text = std::slice::from_raw_parts(value as *const u16, size as usize);
            let name = String::from_utf16_lossy(text).trim_end_matches('\0').trim().to_string();
            (!name.is_empty()).then_some(name)
        }
    }
}
