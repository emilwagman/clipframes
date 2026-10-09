//! cf-history: how long History takes to list its first page when there are many captures.
//!
//!   cf-history          5,000 captures
//!   cf-history 20000    as many as you say
//!
//! Makes the captures in a temporary folder, times the listing, and removes them again.

use clipframes_lib::element::ElementInfo;
use clipframes_lib::round::Round;
use clipframes_lib::store::{self, Stamp};
use std::time::Instant;

fn main() {
    let count: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(5000);
    let root = std::env::temp_dir().join(format!("clipframes-history-bench-{}", std::process::id()));
    let mut round = Round::default();
    for name in ["New invoice", "Export", "Search invoices"] {
        round.add(ElementInfo { app: "Google Chrome".into(), window: "Invoices".into(), role: "Button".into(), name: name.into(), dom_id: "new-invoice".into(), dom_classes: "btn btn-primary".into(), ..Default::default() });
    }
    let made = Instant::now();
    for i in 0..count {
        let stamp = Stamp { year: 2026, month: 1 + (i / 2000) % 12, day: 1 + (i / 80) % 25, hour: (i / 4) % 20, minute: i % 60, second: (i * 7) % 60 };
        store::save(&round, &store::new_folder(&root, stamp), stamp).expect("save");
    }
    println!("made {count} captures in {:.1} s", made.elapsed().as_secs_f64());

    let mut times: Vec<f64> = (0..5)
        .map(|_| {
            let started = Instant::now();
            let (page, total) = store::list(&root, 0, 40);
            assert_eq!((page.len(), total), (40.min(count as usize), count as usize));
            started.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("first page of History with {count} captures: first run {:.0} ms, median {:.0} ms", times[4].max(times[0]), times[2]);
    std::fs::remove_dir_all(&root).expect("clean up");
}
