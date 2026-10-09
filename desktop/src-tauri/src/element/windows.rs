//! Windows: UI Automation. No permission needed. Chromium and Electron expose a page's
//! elements here too: AutomationId carries the DOM id and ClassName the class list.

use super::{ElementInfo, ReadError, Rect};
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

pub fn element_at(x: f64, y: f64) -> Result<ElementInfo, ReadError> {
    let other = |e: uiautomation::Error| ReadError::Other(e.to_string());
    let automation = UIAutomation::new().map_err(other)?;
    let el = automation.element_from_point(Point::new(x as i32, y as i32)).map_err(|_| ReadError::Nothing)?;

    let class = el.get_classname().unwrap_or_default();
    let automation_id = el.get_automation_id().unwrap_or_default();
    let framework = el.get_framework_id().unwrap_or_default();
    let web = framework.eq_ignore_ascii_case("Chrome") || framework.eq_ignore_ascii_case("Gecko");

    let mut info = ElementInfo {
        role: role_of(&el),
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

    // Walk up to the window for its title; the top-level window names the app as well.
    if let Ok(walker) = automation.get_control_view_walker() {
        let mut cur = walker.get_parent(&el).ok();
        let mut depth = 0;
        while let Some(p) = cur {
            if role_of(&p) == "Window" {
                info.window = p.get_name().unwrap_or_default();
            }
            depth += 1;
            if depth > 60 {
                break;
            }
            cur = walker.get_parent(&p).ok();
        }
    }
    Ok(info)
}

/// "ButtonControl" → "Button", to read like the macOS roles.
fn role_of(el: &UIElement) -> String {
    el.get_control_type().map(|t| format!("{:?}", t).trim_end_matches("Control").to_string()).unwrap_or_default()
}
