//! macOS: the Accessibility API. Needs the Accessibility permission.

use super::{ElementInfo, ReadError, Rect};
use accessibility_sys::*;
use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::url::CFURL;
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::{CGPoint, CGSize};
use std::ffi::c_void;
use std::ptr;

pub fn permitted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

pub fn pointer() -> Option<(f64, f64)> {
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let p = CGEvent::new(source).ok()?.location();
    Some((p.x, p.y))
}

pub fn element_at(x: f64, y: f64) -> Result<ElementInfo, ReadError> {
    if !permitted() {
        return Err(ReadError::NotPermitted);
    }
    unsafe {
        let system = AXUIElementCreateSystemWide();
        let mut hit: AXUIElementRef = ptr::null_mut();
        let err = AXUIElementCopyElementAtPosition(system, x as f32, y as f32, &mut hit);
        core_foundation::base::CFRelease(system as CFTypeRef);
        if err != kAXErrorSuccess || hit.is_null() {
            return Err(ReadError::Nothing);
        }
        let el = Element(hit);

        let mut info = ElementInfo {
            role: el.string(kAXRoleAttribute).trim_start_matches("AX").to_string(),
            name: first_non_empty(&[el.string(kAXTitleAttribute), el.string(kAXDescriptionAttribute)]),
            value: el.string(kAXValueAttribute),
            identifier: el.string(kAXIdentifierAttribute),
            dom_id: el.string("AXDOMIdentifier"),
            dom_classes: el.strings("AXDOMClassList").join(" "),
            frame: el.frame(),
            ..Default::default()
        };

        let mut pid: i32 = 0;
        AXUIElementGetPid(el.0, &mut pid);
        info.pid = pid;

        // Walk up for what the element itself doesn't carry: the page URL and the window title.
        let mut cur = el.parent();
        let mut depth = 0;
        while let Some(p) = cur {
            if info.url.is_empty() {
                info.url = p.url("AXURL");
            }
            if info.window.is_empty() && p.string(kAXRoleAttribute) == "AXWindow" {
                info.window = p.string(kAXTitleAttribute);
            }
            if info.app.is_empty() && p.string(kAXRoleAttribute) == "AXApplication" {
                info.app = p.string(kAXTitleAttribute);
            }
            depth += 1;
            if depth > 60 {
                break;
            }
            cur = p.parent();
        }
        Ok(info)
    }
}

fn first_non_empty(v: &[String]) -> String {
    v.iter().find(|s| !s.is_empty()).cloned().unwrap_or_default()
}

/// An owned AXUIElement.
struct Element(AXUIElementRef);

impl Drop for Element {
    fn drop(&mut self) {
        unsafe { core_foundation::base::CFRelease(self.0 as CFTypeRef) }
    }
}

impl Element {
    unsafe fn copy(&self, attribute: &str) -> Option<CFType> {
        let name = CFString::new(attribute);
        let mut value: CFTypeRef = ptr::null();
        let err = AXUIElementCopyAttributeValue(self.0, name.as_concrete_TypeRef(), &mut value);
        if err != kAXErrorSuccess || value.is_null() {
            return None;
        }
        Some(CFType::wrap_under_create_rule(value))
    }

    unsafe fn string(&self, attribute: &str) -> String {
        self.copy(attribute).and_then(|v| v.downcast::<CFString>()).map(|s| s.to_string()).unwrap_or_default()
    }

    unsafe fn strings(&self, attribute: &str) -> Vec<String> {
        let Some(v) = self.copy(attribute) else { return vec![] };
        if v.type_of() != CFArray::<CFType>::type_id() {
            return vec![];
        }
        let array: CFArray<CFType> = CFArray::wrap_under_get_rule(v.as_CFTypeRef() as CFArrayRef);
        array.iter().filter_map(|item| item.downcast::<CFString>()).map(|s| s.to_string()).collect()
    }

    unsafe fn url(&self, attribute: &str) -> String {
        let Some(v) = self.copy(attribute) else { return String::new() };
        if let Some(u) = v.downcast::<CFURL>() {
            return u.get_string().to_string();
        }
        v.downcast::<CFString>().map(|s| s.to_string()).unwrap_or_default()
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

    unsafe fn frame(&self) -> Rect {
        let mut origin = CGPoint::new(0.0, 0.0);
        let mut size = CGSize::new(0.0, 0.0);
        if let Some(v) = self.copy(kAXPositionAttribute) {
            AXValueGetValue(v.as_CFTypeRef() as AXValueRef, kAXValueTypeCGPoint, &mut origin as *mut _ as *mut c_void);
        }
        if let Some(v) = self.copy(kAXSizeAttribute) {
            AXValueGetValue(v.as_CFTypeRef() as AXValueRef, kAXValueTypeCGSize, &mut size as *mut _ as *mut c_void);
        }
        Rect { x: origin.x, y: origin.y, width: size.width, height: size.height }
    }
}

#[allow(dead_code)]
fn _string_ref_is_used(_: CFStringRef) {}
