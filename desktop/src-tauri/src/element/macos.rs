//! macOS: the Accessibility API. Needs the Accessibility permission.
//!
//! The hit-test never asks the system "what is at this point", because the answer would be
//! Clipframes' own overlay. It finds the front window of another app under the point and asks
//! that app, with a short timeout on every question so a hung app can't stall the pointer.

use super::locate::{self, Node, Opened};
use super::{ElementInfo, ReadError, Rect};
use accessibility_sys::*;
use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation::url::CFURL;
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::{CGPoint, CGSize};
use core_graphics::window::{copy_window_info, kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly};
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long one accessibility call may take before it is abandoned.
const TIMEOUT_SECONDS: f32 = 0.25;

const INTERACTIVE: &[&str] = &[
    "Button", "Link", "CheckBox", "RadioButton", "PopUpButton", "MenuButton", "TextField", "TextArea", "ComboBox", "Slider",
    "Tab", "MenuItem", "Cell", "Row", "Switch", "DisclosureTriangle", "Image", "Heading", "SearchField",
];

/// Sets the timeout for every accessibility call this process makes, once. Set on the
/// system-wide element it holds for all of them; set on one element it covers that element
/// only (AXUIElement.h), which left every question after the hit-test (parents, children,
/// each attribute) waiting the default several seconds on an app that had hung.
fn limit_waiting() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| unsafe {
        let system = Element(AXUIElementCreateSystemWide());
        AXUIElementSetMessagingTimeout(system.0, TIMEOUT_SECONDS);
    });
}

pub fn permitted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Has macOS ask the user for the Accessibility permission. Asking this way is what puts
/// Clipframes in the list under Privacy & Security, where the user can switch it on; an app
/// that only checks may not be listed at all. Does nothing once the permission is given.
pub fn ask_permission() {
    unsafe {
        let prompt = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let options = CFDictionary::from_CFType_pairs(&[(prompt.as_CFType(), CFBoolean::true_value().as_CFType())]);
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
    }
}

pub fn pointer() -> Option<(f64, f64)> {
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let p = CGEvent::new(source).ok()?.location();
    Some((p.x, p.y))
}

/// A window of another app, front to back.
#[derive(Debug, Clone)]
pub struct ScreenWindow {
    pub pid: i32,
    pub frame: Rect,
    pub owner: String,
    pub title: String,
}

/// Normal windows of other apps that are on screen, front to back.
pub fn windows() -> Vec<ScreenWindow> {
    let me = std::process::id() as i64;
    let Some(list) = copy_window_info(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, kCGNullWindowID) else {
        return vec![];
    };
    let mut out = Vec::new();
    for raw in list.get_all_values() {
        let d: CFDictionary<CFString, CFType> = unsafe { CFDictionary::wrap_under_get_rule(raw as _) };
        let num = |k: &str| d.find(CFString::new(k)).and_then(|v| v.downcast::<CFNumber>()).and_then(|n| n.to_f64());
        let text = |k: &str| d.find(CFString::new(k)).and_then(|v| v.downcast::<CFString>()).map(|s| s.to_string()).unwrap_or_default();
        if num("kCGWindowLayer") != Some(0.0) {
            continue;
        }
        let Some(pid) = num("kCGWindowOwnerPID") else { continue };
        if pid as i64 == me || num("kCGWindowAlpha").unwrap_or(1.0) <= 0.05 {
            continue;
        }
        let Some(bounds) = d.find(CFString::new("kCGWindowBounds")).and_then(|v| v.downcast::<CFDictionary>()) else { continue };
        let b: CFDictionary<CFString, CFType> = unsafe { CFDictionary::wrap_under_get_rule(bounds.as_concrete_TypeRef()) };
        let side = |k: &str| b.find(CFString::new(k)).and_then(|v| v.downcast::<CFNumber>()).and_then(|n| n.to_f64()).unwrap_or(0.0);
        let frame = Rect { x: side("X"), y: side("Y"), width: side("Width"), height: side("Height") };
        if frame.width <= 40.0 || frame.height <= 40.0 {
            continue;
        }
        out.push(ScreenWindow { pid: pid as i32, frame, owner: text("kCGWindowOwnerName"), title: text("kCGWindowName") });
    }
    out
}

fn window_at(x: f64, y: f64) -> Option<ScreenWindow> {
    windows().into_iter().find(|w| x >= w.frame.x && x < w.frame.x + w.frame.width && y >= w.frame.y && y < w.frame.y + w.frame.height)
}

pub fn element_at(x: f64, y: f64) -> Result<ElementInfo, ReadError> {
    read(x, y, Wake::Keep).map(|(info, _)| info)
}

/// The address of the page at a point, or nothing when there is no page there. With
/// `may_wake` off, a Chromium or Electron app whose page structure is not switched on is
/// left as it is, and has no address to give.
pub fn page_at(x: f64, y: f64, may_wake: bool) -> String {
    read(x, y, if may_wake { Wake::Once } else { Wake::No }).map(|(info, _)| info.url).unwrap_or_default()
}

/// The element, and with it the element itself and what it is in (its page, or failing that
/// its window) for the look a click takes afterwards.
fn read(x: f64, y: f64, wake_how: Wake) -> Result<(ElementInfo, Option<(Element, Element)>), ReadError> {
    let win = window_at(x, y).ok_or(ReadError::Nothing)?;
    wake(win.pid, wake_how);

    // When the app gives nothing, the window itself is still an answer.
    let base = ElementInfo { app: win.owner.clone(), pid: win.pid, window: win.title.clone(), role: "Window".into(), frame: win.frame, ..Default::default() };
    if !permitted() {
        return Err(ReadError::NotPermitted);
    }
    limit_waiting();
    unsafe {
        let app = Element(AXUIElementCreateApplication(win.pid));
        AXUIElementSetMessagingTimeout(app.0, TIMEOUT_SECONDS);
        let mut hit: AXUIElementRef = ptr::null_mut();
        if AXUIElementCopyElementAtPosition(app.0, x as f32, y as f32, &mut hit) != kAXErrorSuccess || hit.is_null() {
            return Ok((base, None));
        }
        let target = best_target(Element(hit));
        let mut around = None;
        let mut info = describe(&target, base, &mut around);
        if info.frame.width < 1.0 {
            info.frame = win.frame;
        }
        Ok((info, around.map(|around| (target, around))))
    }
}

/// Prefer the control a person means: a label inside a button means the button.
unsafe fn best_target(el: Element) -> Element {
    if el.role() == "StaticText" {
        if let Some(p) = el.parent() {
            let styled = !p.string("AXDOMClassList").is_empty() || !p.string("AXDOMIdentifier").is_empty();
            if styled && !["Cell", "Row", "WebArea"].contains(&p.role().as_str()) {
                return p;
            }
        }
    }
    let mut cur = Some(el.retained());
    for _ in 0..4 {
        let Some(c) = cur else { break };
        let role = c.role();
        if ["WebArea", "Window", "ScrollArea", "Application"].contains(&role.as_str()) {
            break;
        }
        if INTERACTIVE.contains(&role.as_str()) || (role != "StaticText" && c.pressable()) {
            return c;
        }
        cur = c.parent();
    }
    el
}

/// `around` is set to the page the element is on (its web area), or failing that its window:
/// what "the others like it" are looked for in.
unsafe fn describe(el: &Element, base: ElementInfo, around: &mut Option<Element>) -> ElementInfo {
    let mut i = base;
    i.role = el.role();
    i.name = [kAXTitleAttribute, kAXDescriptionAttribute, kAXPlaceholderValueAttribute, kAXHelpAttribute]
        .iter()
        .map(|a| el.string(a))
        .find(|s| !s.is_empty())
        .unwrap_or_default();
    let value = el.string(kAXValueAttribute);
    if value.chars().count() < 200 {
        i.value = value;
    }
    let ident = el.string(kAXIdentifierAttribute);
    if !ident.starts_with("_NS:") {
        i.identifier = ident;
    }
    i.dom_id = el.string("AXDOMIdentifier");
    i.dom_classes = el.string("AXDOMClassList");
    if i.name.is_empty() {
        i.inner_text = inner_text(el);
    }
    i.frame = el.frame();

    // Walk up for the page URL, the window title and a short trail of named containers.
    let mut path: Vec<String> = Vec::new();
    let mut cur = el.parent();
    let mut depth = 0;
    let mut in_page = true; // labels above the page (browser chrome) are noise
    while let Some(c) = cur {
        if depth >= 50 {
            break;
        }
        let role = c.role();
        if role == "Application" {
            break;
        }
        if role == "Window" && i.window.is_empty() {
            i.window = c.string(kAXTitleAttribute);
        }
        if (role == "WebArea" || role == "Window") && around.is_none() {
            *around = Some(c.retained());
        }
        if role == "WebArea" {
            if i.url.is_empty() {
                i.url = c.string("AXURL");
            }
            in_page = false;
        } else if in_page && role != "Window" {
            let label = short_label(&c, &role);
            if !label.is_empty() {
                path.insert(0, label);
            }
        }
        cur = c.parent();
        depth += 1;
    }
    let keep = path.len().saturating_sub(4);
    i.path = path.split_off(keep);
    i
}

unsafe fn short_label(el: &Element, role: &str) -> String {
    let name = [kAXTitleAttribute, kAXDescriptionAttribute].iter().map(|a| el.string(a)).find(|s| !s.is_empty()).unwrap_or_default();
    let dom = el.string("AXDOMIdentifier");
    let classes: Vec<String> = el.string("AXDOMClassList").split_whitespace().take(3).map(String::from).collect();
    let mut s = String::new();
    if !name.is_empty() {
        s = format!("{role} \"{}\"", name.chars().take(40).collect::<String>());
    }
    if !dom.is_empty() {
        s = format!("{} #{dom}", if s.is_empty() { role.to_string() } else { s });
    }
    if name.is_empty() && dom.is_empty() && !classes.is_empty() {
        s = format!("{role} .{}", classes.join("."));
    }
    s
}

/// The first few pieces of text inside an unnamed element: a card says what is written on it.
unsafe fn inner_text(el: &Element) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut stack: Vec<(Element, u32)> = el.children().into_iter().rev().map(|c| (c, 0)).collect();
    let mut seen = 0;
    while let Some((c, d)) = stack.pop() {
        if out.len() >= 3 || seen >= 150 {
            break;
        }
        seen += 1;
        if c.role() == "StaticText" {
            let t = c.string(kAXValueAttribute);
            if !t.is_empty() {
                out.push(t.chars().take(40).collect());
            }
        }
        if d < 5 {
            for k in c.children().into_iter().rev() {
                stack.push((k, d + 1));
            }
        }
    }
    out.join(" ")
}

// MARK: Chromium and Electron only build their page tree when asked.

/// The two switches that make a Chromium or Electron app build its page tree.
const SWITCHES: [&str; 2] = ["AXManualAccessibility", "AXEnhancedUserInterface"];

/// An app Clipframes asked for its page tree.
struct Woken {
    /// When it was last looked at during a round.
    at: Instant,
    /// Which of the switches Clipframes turned on itself. One that was already on belongs to
    /// someone else (VoiceOver, a window manager, a keyboard tool) and is never turned off.
    ours: [bool; 2],
}

static WOKEN: Mutex<Option<HashMap<i32, Woken>>> = Mutex::new(None);

/// How much a reading may do to an app that only builds its page tree when asked.
#[derive(Clone, Copy, PartialEq)]
enum Wake {
    /// Leave it as it is.
    No,
    /// Switch the tree on if it is not, and let it go off again five minutes later however
    /// often this is asked: the place watcher, which looks at every change of title.
    Once,
    /// Switch it on and keep it on while this goes on: hovering and clicking in a round.
    Keep,
}

fn wake(pid: i32, how: Wake) {
    if how == Wake::No {
        return;
    }
    let mut guard = WOKEN.lock().unwrap();
    let woken = guard.get_or_insert_with(HashMap::new);
    match woken.get_mut(&pid) {
        Some(known) if how == Wake::Keep => known.at = Instant::now(),
        Some(_) => {}
        None if is_chromium(pid) => {
            woken.insert(pid, Woken { at: Instant::now(), ours: switch_on(pid) });
        }
        None => {}
    }
}

/// Turn it back off for apps not looked at in a while: keeping it on costs them CPU.
pub fn sleep_idle(older_than: Duration) {
    let mut guard = WOKEN.lock().unwrap();
    let Some(woken) = guard.as_mut() else { return };
    let stale: Vec<i32> = woken.iter().filter(|(_, w)| w.at.elapsed() >= older_than).map(|(pid, _)| *pid).collect();
    for pid in stale {
        if let Some(known) = woken.remove(&pid) {
            switch_off(pid, known.ours);
        }
    }
}

/// Turns on the switches that are not on already, and says which ones that was.
fn switch_on(pid: i32) -> [bool; 2] {
    limit_waiting();
    unsafe {
        let app = Element(AXUIElementCreateApplication(pid));
        SWITCHES.map(|switch| {
            // An app that will not say counts as off: Chromium does not answer for its own switch.
            let already = app.copy(switch).is_some_and(|v| is_on(&v));
            if !already {
                AXUIElementSetAttributeValue(app.0, CFString::new(switch).as_concrete_TypeRef(), CFBoolean::true_value().as_CFTypeRef());
            }
            !already
        })
    }
}

fn switch_off(pid: i32, ours: [bool; 2]) {
    limit_waiting();
    unsafe {
        let app = Element(AXUIElementCreateApplication(pid));
        for (switch, _) in SWITCHES.iter().zip(ours).filter(|(_, ours)| *ours) {
            AXUIElementSetAttributeValue(app.0, CFString::new(switch).as_concrete_TypeRef(), CFBoolean::false_value().as_CFTypeRef());
        }
    }
}

/// Whether a switch's value says on: a true, or a number that is not zero.
fn is_on(value: &CFType) -> bool {
    value.downcast::<CFBoolean>().map(bool::from).or_else(|| value.downcast::<CFNumber>().and_then(|n| n.to_i64()).map(|n| n != 0)).unwrap_or(false)
}

/// The app bundle a process runs from, e.g. /Applications/Slack.app.
pub fn bundle_of(pid: i32) -> Option<PathBuf> {
    extern "C" {
        fn proc_pidpath(pid: i32, buffer: *mut c_void, size: u32) -> i32;
    }
    let mut buf = [0u8; 4096];
    let n = unsafe { proc_pidpath(pid, buf.as_mut_ptr() as *mut c_void, buf.len() as u32) };
    if n <= 0 {
        return None;
    }
    let exe = PathBuf::from(String::from_utf8_lossy(&buf[..n as usize]).to_string());
    exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")).map(PathBuf::from)
}

fn is_chromium(pid: i32) -> bool {
    let Some(bundle) = bundle_of(pid) else { return false };
    let name = bundle.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if ["chrome", "chromium", "edge", "brave", "arc", "vivaldi", "opera"].iter().any(|b| name.contains(b)) {
        return true;
    }
    bundle.join("Contents/Frameworks/Electron Framework.framework").exists()
}

// MARK: An owned AXUIElement

/// An attribute's value as text: a string, a URL or a list of strings, trimmed, on one line.
unsafe fn text_of(v: &CFType) -> String {
    if let Some(s) = v.downcast::<CFString>() {
        return s.to_string().trim().replace('\n', " ");
    }
    if let Some(u) = v.downcast::<CFURL>() {
        return u.get_string().to_string();
    }
    if v.type_of() == CFArray::<CFType>::type_id() {
        let array: CFArray<CFType> = CFArray::wrap_under_get_rule(v.as_CFTypeRef() as CFArrayRef);
        return array.iter().filter_map(|item| item.downcast::<CFString>()).map(|s| s.to_string()).collect::<Vec<_>>().join(" ");
    }
    String::new()
}

/// An attribute's value as the elements it lists.
unsafe fn elements_of(v: &CFType) -> Vec<Element> {
    if v.type_of() != CFArray::<CFType>::type_id() {
        return vec![];
    }
    let array: CFArray<CFType> = CFArray::wrap_under_get_rule(v.as_CFTypeRef() as CFArrayRef);
    array
        .iter()
        .filter(|item| item.type_of() == AXUIElementGetTypeID())
        .map(|item| {
            let raw = item.as_CFTypeRef() as AXUIElementRef;
            core_foundation::base::CFRetain(raw as CFTypeRef);
            Element(raw)
        })
        .collect()
}

/// An element as the look through a page meets it (`locate::walk`).
struct AxNode {
    element: Element,
    /// The look is through a page, not a window. A window's own controls are not under the
    /// headings of a page that happens to be shown in it, so from a window pages are not
    /// entered.
    in_page: bool,
}

/// The first pieces of text under these elements, joined: what a heading says when it has no
/// title of its own. At most twenty elements are asked, and none after `left` has run out.
fn text_inside(mut stack: Vec<AxNode>, left: Duration) -> String {
    let started = Instant::now();
    let mut said: Vec<String> = Vec::new();
    stack.reverse();
    for _ in 0..20 {
        let left = left.saturating_sub(started.elapsed());
        let Some(node) = stack.pop().filter(|_| !left.is_zero() && said.len() < 3) else { break };
        let opened = node.open(left);
        if opened.role == "StaticText" && !opened.name.is_empty() {
            said.push(opened.name);
        }
        stack.extend(opened.children.into_iter().rev());
    }
    said.join(" ")
}

/// What is asked about each element on the way, in this order, in one message to the app:
/// asked one by one, a page of a thousand elements would be seven thousand messages.
const ASKED: [&str; 7] = [kAXRoleAttribute, kAXTitleAttribute, kAXDescriptionAttribute, kAXPlaceholderValueAttribute, kAXHelpAttribute, kAXValueAttribute, kAXChildrenAttribute];

impl Node for AxNode {
    fn open(&self, left: Duration) -> Opened<AxNode> {
        let mut opened = Opened { role: String::new(), name: String::new(), heading: None, children: Vec::new() };
        let asked_at = Instant::now();
        unsafe {
            // This element's answer may take what is left of the look's time and no more.
            // Set on the element it holds for this element only (AXUIElement.h), so nothing
            // has to be put back afterwards.
            AXUIElementSetMessagingTimeout(self.element.0, left.as_secs_f32().clamp(0.005, TIMEOUT_SECONDS));
            let asked: Vec<CFString> = ASKED.iter().map(|a| CFString::new(a)).collect();
            let asked = CFArray::from_CFTypes(&asked);
            let mut answers: CFArrayRef = ptr::null();
            // An attribute the element does not have comes back as an error value in its
            // place, which reads as no text and no children below.
            if AXUIElementCopyMultipleAttributeValues(self.element.0, asked.as_concrete_TypeRef(), 0, &mut answers) != kAXErrorSuccess || answers.is_null() {
                return opened;
            }
            let answers: CFArray<CFType> = CFArray::wrap_under_create_rule(answers);
            if answers.len() != ASKED.len() as isize {
                return opened;
            }
            let text = |i: isize| answers.get(i).map(|v| text_of(&v)).unwrap_or_default();
            opened.role = text(0).trim_start_matches("AX").to_string();
            // The same name `describe` gives, or for something without one the value it
            // shows: what `ElementInfo::label` is for the element that was picked.
            let name = (1..=4).map(text).find(|s| !s.is_empty()).unwrap_or_default();
            let value = text(5);
            opened.name = if !name.is_empty() { name } else if value.chars().count() < 200 { value } else { String::new() };
            let inside = || answers.get(6).map(|v| elements_of(&v)).unwrap_or_default().into_iter().map(|element| AxNode { element, in_page: self.in_page });
            if opened.role == "Heading" {
                // A heading says its text as its title, its value, or only in the text inside
                // it. Looking inside is more questions, asked within the same time.
                opened.heading = Some(if opened.name.is_empty() { text_inside(inside().collect(), left.saturating_sub(asked_at.elapsed())) } else { opened.name.clone() });
            }
            // What is inside a piece of text is its lines, one element each in Chromium: half
            // of a page's elements, and never a heading or anything that can be picked.
            let closed = opened.role == "StaticText" || (opened.role == "WebArea" && !self.in_page);
            if !closed {
                opened.children = inside().collect();
            }
        }
        opened
    }

    fn is(&self, other: &AxNode) -> bool {
        unsafe { core_foundation::base::CFEqual(self.element.0 as CFTypeRef, other.element.0 as CFTypeRef) != 0 }
    }
}

struct Element(AXUIElementRef);

impl Drop for Element {
    fn drop(&mut self) {
        unsafe { core_foundation::base::CFRelease(self.0 as CFTypeRef) }
    }
}

impl Element {
    unsafe fn retained(&self) -> Element {
        core_foundation::base::CFRetain(self.0 as CFTypeRef);
        Element(self.0)
    }

    unsafe fn copy(&self, attribute: &str) -> Option<CFType> {
        let name = CFString::new(attribute);
        let mut value: CFTypeRef = ptr::null();
        if AXUIElementCopyAttributeValue(self.0, name.as_concrete_TypeRef(), &mut value) != kAXErrorSuccess || value.is_null() {
            return None;
        }
        Some(CFType::wrap_under_create_rule(value))
    }

    /// A string, a URL or a list of strings as text, trimmed, on one line.
    unsafe fn string(&self, attribute: &str) -> String {
        self.copy(attribute).map(|v| text_of(&v)).unwrap_or_default()
    }

    /// "AXButton" → "Button".
    unsafe fn role(&self) -> String {
        self.string(kAXRoleAttribute).trim_start_matches("AX").to_string()
    }

    unsafe fn parent(&self) -> Option<Element> {
        self.element(kAXParentAttribute)
    }

    /// An attribute whose value is another element.
    unsafe fn element(&self, attribute: &str) -> Option<Element> {
        let v = self.copy(attribute)?;
        if v.type_of() != AXUIElementGetTypeID() {
            return None;
        }
        let raw = v.as_CFTypeRef() as AXUIElementRef;
        core_foundation::base::CFRetain(raw as CFTypeRef);
        Some(Element(raw))
    }

    unsafe fn children(&self) -> Vec<Element> {
        self.copy(kAXChildrenAttribute).map(|v| elements_of(&v)).unwrap_or_default()
    }

    unsafe fn pressable(&self) -> bool {
        let mut names: CFArrayRef = ptr::null();
        if AXUIElementCopyActionNames(self.0, &mut names) != kAXErrorSuccess || names.is_null() {
            return false;
        }
        let array: CFArray<CFString> = CFArray::wrap_under_create_rule(names);
        let found = array.iter().any(|n| n.to_string() == kAXPressAction);
        found
    }

    unsafe fn frame(&self) -> Rect {
        let mut origin = CGPoint::new(0.0, 0.0);
        let mut size = CGSize::new(0.0, 0.0);
        if let Some(v) = self.copy(kAXPositionAttribute) {
            if v.type_of() == AXValueGetTypeID() {
                AXValueGetValue(v.as_CFTypeRef() as AXValueRef, kAXValueTypeCGPoint, &mut origin as *mut _ as *mut c_void);
            }
        }
        if let Some(v) = self.copy(kAXSizeAttribute) {
            if v.type_of() == AXValueGetTypeID() {
                AXValueGetValue(v.as_CFTypeRef() as AXValueRef, kAXValueTypeCGSize, &mut size as *mut _ as *mut c_void);
            }
        }
        Rect { x: origin.x, y: origin.y, width: size.width, height: size.height }
    }
}

/// Every reading here is already the full one.
pub fn element_full_at(x: f64, y: f64) -> Result<ElementInfo, ReadError> {
    element_at(x, y)
}

pub fn foreground() -> Option<super::Foreground> {
    let mut front = windows().into_iter().next()?;
    // The window list only gives titles to an app with the Screen Recording permission.
    // Without it every title is empty, and another tab or page in the same app would look
    // like no change at all. The app's own answer needs only the Accessibility permission.
    if front.title.is_empty() && permitted() {
        front.title = focused_title(front.pid);
    }
    Some(super::Foreground { app: front.owner, title: front.title, pid: front.pid, frame: front.frame })
}

/// The title of the window an app has the keyboard in, as the app itself reports it.
fn focused_title(pid: i32) -> String {
    limit_waiting();
    unsafe {
        let app = Element(AXUIElementCreateApplication(pid));
        app.element(kAXFocusedWindowAttribute).map(|window| window.string(kAXTitleAttribute)).unwrap_or_default()
    }
}

/// The reading for a click: the element, and which one it is and under what heading.
pub fn element_picked_at(x: f64, y: f64) -> Result<super::Picked, ReadError> {
    let (info, parts) = read(x, y, Wake::Keep)?;
    let Some((target, around)) = parts else { return Ok(super::Picked { info, later: None }) };
    let (role, label, on_page) = (info.role.clone(), info.label().to_string(), !info.url.is_empty());
    Ok(super::Picked::new(info, move || unsafe {
        // A page was found on the way up exactly when the element has an address.
        let in_page = on_page || around.role() == "WebArea";
        let target = AxNode { element: target, in_page };
        locate::walk(AxNode { element: around, in_page }, &target, &role, &label, &locate::BUDGET)
    }))
}
