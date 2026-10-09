//! cf-probe: prints the element under the pointer once a second, as the reference would name it
//! and as JSON. The quickest way to see what a system and an app expose.
//!
//!   cf-probe            watch the pointer until Ctrl-C
//!   cf-probe 5          five readings, then stop
//!   cf-probe at 640 400 one reading at a point

use clipframes_lib::element::{self, ReadError};
use std::{env, thread, time::Duration};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if !element::permitted() {
        eprintln!("Not permitted to read other apps yet (on macOS: System Settings › Privacy & Security › Accessibility).");
    }
    if args.first().map(String::as_str) == Some("at") {
        let x = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let y = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        read(x, y);
        return;
    }
    #[cfg(windows)]
    if args.first().map(String::as_str) == Some("up") {
        let x = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let y = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        if args.iter().any(|a| a == "wake") {
            println!("wake: {}", element::wake_at(x as f64, y as f64));
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }
        up(x, y);
        return;
    }
    let count: Option<u32> = args.first().and_then(|s| s.parse().ok());
    let mut n = 0;
    loop {
        match element::pointer() {
            Some((x, y)) => read(x, y),
            None => eprintln!("Can't read the pointer position on this system yet."),
        }
        n += 1;
        if count.is_some_and(|c| n >= c) {
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn read(x: f64, y: f64) {
    match element::element_at(x, y) {
        Ok(mut e) => {
            // A terminal or an editor hands over its whole text as the value; show the start of it.
            if e.value.chars().count() > 80 {
                e.value = e.value.chars().take(80).collect::<String>().replace('\n', " ") + "…";
            }
            let selector = e.selector();
            println!("({x:.0}, {y:.0})  {}{}", e.headline(), if selector.is_empty() { String::new() } else { format!("  ({selector})") });
            println!("{}", serde_json::to_string(&e).unwrap_or_default());
        }
        Err(ReadError::Nothing) => println!("({x:.0}, {y:.0})  nothing there"),
        Err(e) => println!("({x:.0}, {y:.0})  {}", serde_json::to_string(&e).unwrap_or_default()),
    }
}

/// `cf-probe up X Y` (Windows): the element at a point and everything above it in the raw
/// tree, with the properties a selector could be built from.
#[cfg(windows)]
fn up(x: i32, y: i32) {
    use uiautomation::types::{Point, UIProperty};
    use uiautomation::UIAutomation;
    let automation = UIAutomation::new().expect("UI Automation");
    let walker = automation.get_raw_view_walker().expect("raw walker");
    let mut cur = automation.element_from_point(Point::new(x, y)).ok();
    let mut depth = 0;
    while let Some(el) = cur {
        let text = |p: UIProperty| el.get_property_value(p).map(|v| v.to_string()).unwrap_or_default();
        println!(
            "{depth:>2} {:?} name={:?} class={:?} id={:?} fw={:?} aria-role={:?}",
            el.get_control_type().ok(),
            el.get_name().unwrap_or_default().chars().take(40).collect::<String>(),
            el.get_classname().unwrap_or_default(),
            el.get_automation_id().unwrap_or_default(),
            el.get_framework_id().unwrap_or_default(),
            text(UIProperty::AriaRole),
        );
        depth += 1;
        if depth > 30 {
            break;
        }
        cur = walker.get_parent(&el).ok();
    }
}
