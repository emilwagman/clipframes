# Performance record

Each entry: what was measured, on which machine, how to repeat it. Budgets are in ARCHITECTURE.md.

## 2026-10-09: reading the element under the pointer (macOS)

MacBook Pro (this hub), macOS, release build. `cf-bench 1728 1117 4`: a 16 × 10 grid over the
whole screen, 4 rounds, the first one discarded as warm-up. 480 readings.

| Where | n | Median | p95 | p99 | Max | Over one 60 Hz frame |
|---|---|---|---|---|---|---|
| All | 480 | 1.62 ms | 5.74 ms | 8.26 ms | 8.86 ms | 0.0% |
| Ghostty (native) | 288 | 1.74 ms | 4.92 ms | 5.91 ms | 7.19 ms | 0.0% |
| Slack (Electron) | 24 | 5.74 ms | 8.75 ms | 8.86 ms | 8.86 ms | 0.0% |
| Spotify | 24 | 4.70 ms | 8.48 ms | 8.81 ms | 8.81 ms | 0.0% |

Within budget (median under 5 ms, p95 under 16 ms). Not yet measured: Windows, a very large
web page, many windows, and the app as a whole (memory, idle CPU, download size).

## 2026-10-09: the app on Windows

Windows 11 desktop (fleet-win), 3840 × 2160 at 150%, release build, binary 3.4 MB.
Element readings: `cf-bench 3840 2160 4` (before the reader kept one UI Automation connection
and stopped walking parents for the window title; to be measured again).

| Where | n | Median | p95 | p99 | Max | Over one 60 Hz frame |
|---|---|---|---|---|---|---|
| All | 480 | 2.96 ms | 13.41 ms | 15.19 ms | 16.69 ms | 0.0% |

The app, measured with `Get-Process` over 30 s in each state:

| State | Core process | Web view processes | CPU |
|---|---|---|---|
| Idle, just started | 15.3 MB working set, 4.9 MB private | none | 0.00% |
| Idle, after a round and the 90 s keep-warm | 35.6 MB, 11.9 MB private | none | 0.00% |
| Bar hidden, kept warm | 35.9 MB | 6 processes, 332 MB | 0.00% |
| Round open, pointer still | 67.1 MB | 8 processes, 453 MB | 0.00% |
| Round open, pointer moving 125 times a second | | | core 15.9%, web views 7.2% of one core |

Web view memory is the sum of working sets, which counts shared pages more than once; it is
still the reason no web view exists while idle. The first build kept the bar as a hidden
window and held about 330 MB doing nothing.

Opening the bar:

| | Picking works after | Bar drawn after | All windows after |
|---|---|---|---|
| Cold (no web view running) | 500–890 ms | 600–970 ms | 780–1140 ms |
| Warm (within 90 s of closing) | 17 ms | at once | 266 ms |

Within budget: idle memory (under 100 MB) and idle CPU (under 0.5%). Not within what "instant"
should mean: the cold open. Starting WebView2 is nearly all of it. Open questions are in
ARCHITECTURE.md under "Idle size against opening speed".

Found while testing, fixed: a hidden web view at idle; Esc not reaching the keyboard hook while
the comment box had the keyboard; a plain `cargo build` loading the interface from a dev server.
Not measured yet: macOS as a whole app, a very large page, many windows, 200 picks, 8 h idle.
