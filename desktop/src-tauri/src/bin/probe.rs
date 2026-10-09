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
