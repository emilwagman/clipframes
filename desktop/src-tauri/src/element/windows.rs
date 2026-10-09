//! Windows: UI Automation. No permission needed. Chromium and Electron expose a page's
//! elements here too: AutomationId carries the DOM id and ClassName the class list.

use super::locate::{self, Located, Node, Opened, Seen};
use super::{ElementInfo, ReadError, Rect};
use uiautomation::patterns::UIValuePattern;
use uiautomation::core::{UICacheRequest, UICondition};
use uiautomation::types::{ControlType, HeadingLevel, Point, TreeScope, UIProperty};
use uiautomation::variants::Variant;
use uiautomation::{UIAutomation, UIElement, UITreeWalker};

pub fn permitted() -> bool {
    true
}

pub fn ask_permission() {}

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
        full(automation, x, y).map(|read| read.info)
    })
}

/// The reading for a click: the element, and which one it is and under what heading.
pub fn element_picked_at(x: f64, y: f64) -> Result<ElementInfo, ReadError> {
    AUTOMATION.with(|automation| {
        let automation = automation.as_ref().ok_or(ReadError::Other("UI Automation is not available.".into()))?;
        let mut read = full(automation, x, y)?;
        if let Some(found) = read.around.as_ref().and_then(|around| locate_in(automation, around, &read.element, &read.info)) {
            read.info.occurrence = found.occurrence;
            read.info.heading = found.heading;
        }
        Ok(read.info)
    })
}

struct Read {
    info: ElementInfo,
    element: UIElement,
    /// The page the element is on (its document), or failing that its window: what "the
    /// others like it" are looked for in.
    around: Option<UIElement>,
}

fn full(automation: &UIAutomation, x: f64, y: f64) -> Result<Read, ReadError> {
    {
        let el = automation.element_from_point(Point::new(x as i32, y as i32)).map_err(|_| ReadError::Nothing)?;
        let mut info = describe(&el, x as i32, y as i32);
        let mut around: Option<UIElement> = None;
        // The raw tree: the simplified one leaves out plain containers, and in a page those
        // are the elements that carry the ids and classes.
        let Ok(walker) = automation.get_raw_view_walker() else { return Ok(Read { info, element: el, around }) };

        let borrow = info.role == "Text" && info.dom_id.is_empty() && info.dom_classes.is_empty();
        let mut parent = walker.get_parent(&el).ok();
        let mut steps = 0;
        while let Some(p) = parent {
            let role = role_of(&p);
            if role == "Window" || steps > 40 {
                if role == "Window" && around.is_none() {
                    around = Some(p);
                }
                break;
            }
            let web = is_web(&p);
            if role == "Document" && web && around.is_none() {
                around = Some(p.clone());
            }
            let (id, class) = if web && role != "Document" { (p.get_automation_id().unwrap_or_default(), p.get_classname().unwrap_or_default()) } else { Default::default() };
            if borrow && steps < 3 && info.dom_id.is_empty() && info.dom_classes.is_empty() {
                // A piece of text has no id or class of its own; the element holding it does.
                info.dom_id = id.clone();
                info.dom_classes = class;
            }
            if role == "Document" {
                if info.url.is_empty() && web {
                    info.url = p.get_pattern::<UIValuePattern>().and_then(|v| v.get_value()).unwrap_or_default();
                }
            } else if role != "Pane" && info.path.len() < 4 {
                // Named by what it says, or failing that by its id.
                let name = p.get_name().unwrap_or_default();
                let label = if !name.is_empty() && name.chars().count() <= 40 { name } else if !id.is_empty() { format!("#{id}") } else { String::new() };
                if !label.is_empty() && label != info.name && label != format!("#{}", info.dom_id) {
                    info.path.insert(0, label);
                }
            }
            steps += 1;
            parent = walker.get_parent(&p).ok();
        }
        Ok(Read { info, element: el, around })
    }
}

/// Which of the elements that read the same this one is, and the heading it is under.
///
/// An element on a page is looked for in its document, anything else in its window without
/// the pages shown in it: a toolbar button is not "2nd of 2" because a page has a button of
/// the same name, and is not under a page's heading.
fn locate_in(automation: &UIAutomation, around: &UIElement, el: &UIElement, info: &ElementInfo) -> Option<Located> {
    // Every question below waits at most the budget's time, and the limits are put back when
    // this returns.
    let _limits = Limits::set(automation, locate::BUDGET.time);
    if role_of(around) == "Document" {
        in_document(automation, around, el, info)
    } else {
        in_window(automation, around, el, info)
    }
}

/// In a page: not a walk from here, where every step is a call into the browser and a page is
/// thousands of steps. The browser is asked once for the two kinds of element that matter,
/// the headings and the ones with this element's control type and name, and hands them back
/// in document order with what is needed already read. It does the looking in its own
/// process. How many elements it looks at cannot be limited from here; what is limited is how
/// long the answer is waited for, and an answer of more matches than the budget's count is
/// not used.
///
/// A heading is an element whose ARIA role is "heading" (what browsers and web views say for
/// h1 to h6 and role=heading) or that has a heading level.
fn in_document(automation: &UIAutomation, around: &UIElement, el: &UIElement, info: &ElementInfo) -> Option<Located> {
    let kind = el.get_control_type().ok()?;
    let name = info.label();
    let is = |property: UIProperty, value: Variant| automation.create_property_condition(property, value, None);

    let mut headings = is(UIProperty::AriaRole, Variant::from("heading")).ok()?;
    // Heading levels are not known to Windows before 2018: then the ARIA role alone decides.
    let by_level = is(UIProperty::HeadingLevel, Variant::from(HeadingLevel::HeadingLevelNone as i32)).and_then(|none| automation.create_not_condition(none));
    if let Ok(by_level) = by_level {
        headings = automation.create_or_condition(headings, by_level).ok()?;
    }
    let mut alike = automation.create_and_condition(is(UIProperty::ControlType, Variant::from(kind as i32)).ok()?, is(UIProperty::Name, Variant::from(name)).ok()?).ok()?;
    if name.is_empty() {
        // Nothing to count, but the element itself must be in the answer to know which
        // headings come before it. Its id and class keep the unnamed ones few.
        for (property, value) in [(UIProperty::AutomationId, el.get_automation_id().unwrap_or_default()), (UIProperty::ClassName, el.get_classname().unwrap_or_default())] {
            alike = automation.create_and_condition(alike, is(property, Variant::from(value)).ok()?).ok()?;
        }
    }
    let wanted = automation.create_or_condition(headings, alike).ok()?;
    let cache = cache_of(automation, &[UIProperty::Name, UIProperty::ControlType, UIProperty::AriaRole, UIProperty::BoundingRectangle])?;
    let found = find_in_order(around, wanted, &cache)?;
    if found.len() > locate::BUDGET.nodes {
        return None;
    }

    let alike = |e: &UIElement| e.get_cached_control_type().is_ok_and(|k| k == kind) && e.get_cached_name().unwrap_or_default() == name;
    let heading = |e: &UIElement| {
        let by_role = e.get_cached_property_value(UIProperty::AriaRole).ok().and_then(|v| v.get_string().ok()).is_some_and(|role| role.eq_ignore_ascii_case("heading"));
        let by_level = e.get_cached_heading_level().is_ok_and(|level| level != HeadingLevel::HeadingLevelNone);
        (by_role || by_level).then(|| e.get_cached_name().unwrap_or_default())
    };
    // Which of them is the element that was clicked: one in the same place, confirmed by
    // asking. Only those in the same place are asked about.
    let frame = el.get_bounding_rectangle().ok()?;
    let target = found.iter().position(|e| alike(e) && e.get_cached_bounding_rectangle().is_ok_and(|r| r == frame) && automation.compare_elements(e, el).unwrap_or(false))?;
    // A heading after the element only matters when none comes before it, and then only the
    // first one, if it is inside the element.
    let none_before = !found[..target].iter().any(|e| heading(e).is_some());
    let first_after = found.iter().enumerate().skip(target + 1).find(|(_, e)| heading(e).is_some()).map(|(i, _)| i).filter(|_| none_before);
    let walker = automation.get_raw_view_walker().ok();
    let seen = found.iter().enumerate().map(|(i, e)| Seen {
        same: !name.is_empty() && alike(e),
        target: i == target,
        heading: heading(e),
        inside: Some(i) == first_after && walker.as_ref().is_some_and(|w| within(automation, w, e, el)),
    });
    locate::place(seen.collect::<Vec<_>>())
}

/// A request that reads these properties (and the heading level, where Windows knows it) with
/// every element it returns, from every element and not only the ones the simplified view keeps.
fn cache_of(automation: &UIAutomation, properties: &[UIProperty]) -> Option<UICacheRequest> {
    let cache = automation.create_cache_request().ok()?;
    for property in properties {
        cache.add_property(*property).ok()?;
    }
    let _ = cache.add_property(UIProperty::HeadingLevel);
    cache.set_tree_filter(automation.create_true_condition().ok()?).ok()?;
    Some(cache)
}

/// Everything under `around` that meets `wanted`, in document order. Where Windows has the
/// call that names the order (pre-order, first child first) it is used; the older `FindAll`
/// goes through the tree the same way without saying so.
fn find_in_order(around: &UIElement, wanted: UICondition, cache: &UICacheRequest) -> Option<Vec<UIElement>> {
    use windows::core::Interface;
    use windows::Win32::UI::Accessibility::{IUIAutomationCacheRequest, IUIAutomationCondition, IUIAutomationElement, IUIAutomationElement7, TreeScope_Descendants, TreeTraversalOptions_Default};
    let raw: &IUIAutomationElement = around.as_ref();
    let Ok(newer) = raw.cast::<IUIAutomationElement7>() else {
        return around.find_all_build_cache(TreeScope::Descendants, &wanted, cache).ok();
    };
    let condition: IUIAutomationCondition = wanted.into();
    let request: &IUIAutomationCacheRequest = cache.as_ref();
    let in_order = unsafe {
        newer.FindAllWithOptionsBuildCache(TreeScope_Descendants, &condition, request, TreeTraversalOptions_Default, None).ok().and_then(|found| (0..found.Length().ok()?).map(|i| found.GetElement(i).ok().map(UIElement::from)).collect::<Option<Vec<_>>>())
    };
    // Should the newer call refuse, the older one is known to work.
    in_order.or_else(|| around.find_all_build_cache(TreeScope::Descendants, &UICondition::from(condition), cache).ok())
}

/// In a window: a walk, one question per element, each answering with the element's children
/// and what is needed about them already read. A page shown in the window is not entered.
/// Here every element looked at is counted, and the walk stops at the budget.
///
/// A heading is an element with a heading level, which is how native apps mark one.
fn in_window(automation: &UIAutomation, around: &UIElement, el: &UIElement, info: &ElementInfo) -> Option<Located> {
    let cache = cache_of(automation, &[UIProperty::Name, UIProperty::ControlType, UIProperty::BoundingRectangle, UIProperty::FrameworkId])?;
    let all = automation.create_true_condition().ok()?;
    let with = Asking { automation, cache: &cache, all: &all };
    let live = |e: &UIElement| UiaNode {
        with: &with,
        element: e.clone(),
        kind: e.get_control_type().ok(),
        name: e.get_name().unwrap_or_default(),
        frame: e.get_bounding_rectangle().ok(),
        heading: e.get_heading_level().is_ok_and(|level| level != HeadingLevel::HeadingLevelNone),
        page: false,
    };
    locate::walk(live(around), &live(el), &info.role, info.label(), &locate::BUDGET)
}

/// What every question of the walk through a window is asked with.
struct Asking<'a> {
    automation: &'a UIAutomation,
    cache: &'a UICacheRequest,
    all: &'a UICondition,
}

/// An element as the walk through a window meets it, with what was read along with it.
struct UiaNode<'a> {
    with: &'a Asking<'a>,
    element: UIElement,
    kind: Option<ControlType>,
    name: String,
    frame: Option<uiautomation::types::Rect>,
    heading: bool,
    /// A page shown in the window: not entered.
    page: bool,
}

impl Node for UiaNode<'_> {
    fn open(&self, _left: std::time::Duration) -> Opened<Self> {
        // A piece of text and a picture hold nothing; a page is another matter (see above).
        let closed = self.page || matches!(self.kind, Some(ControlType::Text) | Some(ControlType::Image));
        let children = if closed { Vec::new() } else { self.element.find_all_build_cache(TreeScope::Children, self.with.all, self.with.cache).unwrap_or_default() };
        let children = children.into_iter().map(|element| {
            let kind = element.get_cached_control_type().ok();
            let web = element.get_cached_framework_id().is_ok_and(|f| f.eq_ignore_ascii_case("Chrome") || f.eq_ignore_ascii_case("Gecko"));
            UiaNode {
                with: self.with,
                kind,
                name: element.get_cached_name().unwrap_or_default(),
                frame: element.get_cached_bounding_rectangle().ok(),
                heading: element.get_cached_heading_level().is_ok_and(|level| level != HeadingLevel::HeadingLevelNone),
                page: web && kind == Some(ControlType::Document),
                element,
            }
        });
        Opened { role: self.kind.map(role_name).unwrap_or_default(), name: self.name.clone(), heading: self.heading.then(|| self.name.clone()), children: children.collect() }
    }

    fn is(&self, other: &Self) -> bool {
        // Asking costs a call, so only an element that looks the same and is in the same place is asked about.
        self.kind == other.kind && self.name == other.name && self.frame == other.frame && self.with.automation.compare_elements(&self.element, &other.element).unwrap_or(false)
    }
}

/// Whether `inner` sits inside `outer`, going up at most as far as a page is deep.
fn within(automation: &UIAutomation, walker: &UITreeWalker, inner: &UIElement, outer: &UIElement) -> bool {
    let mut parent = walker.get_parent(inner).ok();
    for _ in 0..40 {
        let Some(p) = parent else { return false };
        if automation.compare_elements(&p, outer).unwrap_or(false) {
            return true;
        }
        parent = walker.get_parent(&p).ok();
    }
    false
}

/// How long this thread's questions to another app may take, shortened for as long as this
/// lives and then put back. Windows has two limits: one for being handed an element and one
/// for being told about an element. Which of them governs a search is not written down, so
/// both are set. The other app is not stopped when the wait ends; it finishes on its own.
struct Limits {
    automation: Option<windows::Win32::UI::Accessibility::IUIAutomation2>,
    before: (u32, u32),
}

impl Limits {
    fn set(automation: &UIAutomation, longest: std::time::Duration) -> Limits {
        use windows::core::Interface;
        use windows::Win32::UI::Accessibility::{IUIAutomation, IUIAutomation2};
        let raw: &IUIAutomation = automation.as_ref();
        let Ok(newer) = raw.cast::<IUIAutomation2>() else { return Limits { automation: None, before: (0, 0) } };
        let ms = longest.as_millis() as u32;
        unsafe {
            // The system's own defaults, should it not say what they are now.
            let before = (newer.ConnectionTimeout().unwrap_or(2000), newer.TransactionTimeout().unwrap_or(20000));
            let _ = newer.SetConnectionTimeout(ms);
            let _ = newer.SetTransactionTimeout(ms);
            Limits { automation: Some(newer), before }
        }
    }
}

impl Drop for Limits {
    fn drop(&mut self) {
        if let Some(automation) = &self.automation {
            unsafe {
                let _ = automation.SetConnectionTimeout(self.before.0);
                let _ = automation.SetTransactionTimeout(self.before.1);
            }
        }
    }
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
    el.get_control_type().map(role_name).unwrap_or_default()
}

fn role_name(kind: ControlType) -> String {
    format!("{kind:?}").trim_end_matches("Control").to_string()
}

pub fn sleep_idle(_older_than: std::time::Duration) {}

pub fn page_at(x: f64, y: f64, _may_wake: bool) -> String {
    element_full_at(x, y).map(|e| e.url).unwrap_or_default()
}

pub fn foreground() -> Option<super::Foreground> {
    win::foreground()
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

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    #[repr(C)]
    #[derive(Default)]
    struct Bounds {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> Handle;
        fn GetWindowRect(hwnd: Handle, rect: *mut Bounds) -> i32;
    }

    /// The window in front, unless it is one of Clipframes' own.
    pub fn foreground() -> Option<crate::element::Foreground> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == 0 || pid == std::process::id() {
                return None;
            }
            let mut text = [0u16; 512];
            let n = GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32).max(0) as usize;
            let mut b = Bounds::default();
            GetWindowRect(hwnd, &mut b);
            let frame = crate::element::Rect { x: b.left as f64, y: b.top as f64, width: (b.right - b.left) as f64, height: (b.bottom - b.top) as f64 };
            Some(crate::element::Foreground { app: app_name(pid), title: String::from_utf16_lossy(&text[..n]), pid: pid as i32, frame })
        }
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
