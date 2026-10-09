//! macOS: the Accessibility API. Needs the Accessibility permission.
//!
//! The hit-test never asks the system "what is at this point", because the answer would be
//! Clipframes' own overlay. It finds the front window of another app under the point and asks
//! that app, with a short timeout so a hung app can't stall the pointer.

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
    let win = window_at(x, y).ok_or(ReadError::Nothing)?;
    wake(win.pid);

    // When the app gives nothing, the window itself is still an answer.
    let base = ElementInfo { app: win.owner.clone(), pid: win.pid, window: win.title.clone(), role: "Window".into(), frame: win.frame, ..Default::default() };
    if !permitted() {
        return Err(ReadError::NotPermitted);
    }
    unsafe {
        let app = Element(AXUIElementCreateApplication(win.pid));
        AXUIElementSetMessagingTimeout(app.0, TIMEOUT_SECONDS);
        let mut hit: AXUIElementRef = ptr::null_mut();
        if AXUIElementCopyElementAtPosition(app.0, x as f32, y as f32, &mut hit) != kAXErrorSuccess || hit.is_null() {
            return Ok(base);
        }
        let target = best_target(Element(hit));
        let mut info = describe(&target, base);
        if info.frame.width < 1.0 {
            info.frame = win.frame;
        }
        Ok(info)
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

unsafe fn describe(el: &Element, base: ElementInfo) -> ElementInfo {
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

static WOKEN: Mutex<Option<HashMap<i32, Instant>>> = Mutex::new(None);

fn wake(pid: i32) {
    let mut guard = WOKEN.lock().unwrap();
    let woken = guard.get_or_insert_with(HashMap::new);
    if !woken.contains_key(&pid) {
        if !is_chromium(pid) {
            return;
        }
        set_tree(pid, true);
    }
    woken.insert(pid, Instant::now());
}

/// Turn it back off for apps not looked at in a while: keeping it on costs them CPU.
pub fn sleep_idle(older_than: Duration) {
    let mut guard = WOKEN.lock().unwrap();
    let Some(woken) = guard.as_mut() else { return };
    let stale: Vec<i32> = woken.iter().filter(|(_, at)| at.elapsed() >= older_than).map(|(pid, _)| *pid).collect();
    for pid in stale {
        set_tree(pid, false);
        woken.remove(&pid);
    }
}

fn set_tree(pid: i32, on: bool) {
    unsafe {
        let app = Element(AXUIElementCreateApplication(pid));
        let value = if on { CFBoolean::true_value() } else { CFBoolean::false_value() };
        for attribute in ["AXManualAccessibility", "AXEnhancedUserInterface"] {
            AXUIElementSetAttributeValue(app.0, CFString::new(attribute).as_concrete_TypeRef(), value.as_CFTypeRef());
        }
    }
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
        let Some(v) = self.copy(attribute) else { return String::new() };
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

    /// "AXButton" → "Button".
    unsafe fn role(&self) -> String {
        self.string(kAXRoleAttribute).trim_start_matches("AX").to_string()
    }

    unsafe fn parent(&self) -> Option<Element> {
        let v = self.copy(kAXParentAttribute)?;
        if v.type_of() != AXUIElementGetTypeID() {
            return None;
        }
        let raw = v.as_CFTypeRef() as AXUIElementRef;
        core_foundation::base::CFRetain(raw as CFTypeRef);
        Some(Element(raw))
    }

    unsafe fn children(&self) -> Vec<Element> {
        let Some(v) = self.copy(kAXChildrenAttribute) else { return vec![] };
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
    let front = windows().into_iter().next()?;
    Some(super::Foreground { app: front.owner, title: front.title, pid: front.pid, frame: front.frame })
}
