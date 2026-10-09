//! cf-pick: runs a picking round in the terminal for a few seconds and prints what it sees.
//! While it runs, clicks are taken by Clipframes and do not reach other apps. Esc ends it.
//!
//!   cf-pick        8 seconds
//!   cf-pick 20     20 seconds

use clipframes_lib::picker::{Event, Picker};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() {
    let seconds: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(8);
    let done = Arc::new(AtomicBool::new(false));
    let (hovers, picks) = (Arc::new(AtomicU32::new(0)), Arc::new(AtomicU32::new(0)));

    let picker = {
        let (done, hovers, picks) = (done.clone(), hovers.clone(), picks.clone());
        Picker::start(move |event| match event {
            Event::Hover { x, y, element, read_ms } => {
                hovers.fetch_add(1, Ordering::Relaxed);
                let what = element.map(|e| e.headline()).unwrap_or_else(|| "nothing".into());
                println!("hover ({x:.0}, {y:.0})  {:.1} ms  {}", read_ms, what.chars().take(90).collect::<String>());
            }
            Event::Pick { x, y, element } => {
                picks.fetch_add(1, Ordering::Relaxed);
                let selector = element.selector();
                println!("PICK  ({x:.0}, {y:.0})  {}{}", element.headline(), if selector.is_empty() { String::new() } else { format!("  ({selector})") });
            }
            Event::Cancel => {
                println!("cancelled with Esc");
                done.store(true, Ordering::SeqCst);
            }
        })
    };
    let picker = match picker {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let started = Instant::now();
    while !done.load(Ordering::SeqCst) && started.elapsed() < Duration::from_secs(seconds) {
        std::thread::sleep(Duration::from_millis(50));
    }
    drop(picker);
    println!("ended: {} hovers, {} picks", hovers.load(Ordering::Relaxed), picks.load(Ordering::Relaxed));
}
