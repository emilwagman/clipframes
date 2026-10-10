//! cf-bench: how long reading an element takes on this machine, right now.
//!
//!   cf-bench                 a 16 × 10 grid over a 1440 × 900 area, 3 rounds
//!   cf-bench 2560 1440 5     the area to cover and the number of rounds
//!
//! The overlay asks for the element under the pointer as the pointer moves. A frame at 60 Hz
//! is 16.7 ms; readings slower than that are answered late, never drawn late, because the
//! reading runs off the UI thread. This prints the distribution so that claim can be checked.
//! Then it times the fuller reading made once for a click.

use clipframes_lib::element;
use std::env;
use std::time::Instant;

fn main() {
    let args: Vec<f64> = env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    let width = args.first().copied().unwrap_or(1440.0);
    let height = args.get(1).copied().unwrap_or(900.0);
    let rounds = args.get(2).copied().unwrap_or(3.0) as usize;
    let (cols, rows) = (16, 10);

    if !element::permitted() {
        eprintln!("Not permitted to read other apps; timings would only measure the refusal.");
    }

    let mut ms: Vec<f64> = Vec::new();
    let mut by_app: std::collections::BTreeMap<String, Vec<f64>> = Default::default();
    let mut nothing = 0;
    for round in 0..rounds {
        for r in 0..rows {
            for c in 0..cols {
                let x = (c as f64 + 0.5) * width / cols as f64;
                let y = (r as f64 + 0.5) * height / rows as f64;
                let t = Instant::now();
                let result = element::element_at(x, y);
                let took = t.elapsed().as_secs_f64() * 1000.0;
                // The first round warms caches and wakes apps; it is reported separately.
                if round > 0 || rounds == 1 {
                    ms.push(took);
                    match result {
                        Ok(e) => by_app.entry(if e.app.is_empty() { "(unnamed app)".into() } else { e.app }).or_default().push(took),
                        Err(_) => nothing += 1,
                    }
                }
            }
        }
    }

    println!("{} readings over {width:.0} × {height:.0} ({nothing} with no element)", ms.len());
    report("all", &mut ms);
    for (app, mut v) in by_app {
        report(&app, &mut v);
    }

    // What a click reads: the element, which is what the comment box waits for, and then
    // which one of several it is and under what heading, a look through the whole page or
    // window that nothing waits for. That look may take up to its budget and then says
    // nothing, so this also counts how often it had something to say.
    let (mut click, mut extra, mut said) = (Vec::new(), Vec::new(), 0);
    for r in 0..rows {
        for c in (0..cols).step_by(2) {
            let x = (c as f64 + 0.5) * width / cols as f64;
            let y = (r as f64 + 0.5) * height / rows as f64;
            let t = Instant::now();
            let mut picked = element::element_picked_at(x, y);
            let shown = t.elapsed().as_secs_f64() * 1000.0;
            let t = Instant::now();
            let located = picked.as_mut().ok().and_then(|p| p.whereabouts());
            click.push(shown);
            extra.push(t.elapsed().as_secs_f64() * 1000.0);
            said += located.is_some_and(|l| l.occurrence.is_some() || l.heading.is_some()) as usize;
        }
    }
    let budget = &element::locate::BUDGET;
    println!("the reading for a click, at {} points (budget for the look through the page: {} elements or {} ms); {said} said which one or under what heading", click.len(), budget.nodes, budget.time.as_millis());
    report("click, until it can be shown", &mut click);
    report("click, the look afterwards", &mut extra);
}

fn report(label: &str, v: &mut Vec<f64>) {
    if v.is_empty() {
        return;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let at = |p: f64| v[((v.len() as f64 - 1.0) * p).round() as usize];
    let over_frame = v.iter().filter(|t| **t > 16.7).count();
    println!(
        "  {label:<28} n={:<4} median {:>6.2} ms   p95 {:>6.2}   p99 {:>6.2}   max {:>7.2}   over one frame: {:.1}%",
        v.len(),
        at(0.5),
        at(0.95),
        at(0.99),
        v[v.len() - 1],
        100.0 * over_frame as f64 / v.len() as f64
    );
}
