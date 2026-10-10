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

## 2026-10-09: stress run on Windows

`tools/win-stress.ps1` in the desktop session of fleet-win, with `demo/northwind` maximised in
Chrome. Real input events: 200 clicks 60 ms apart in one round, then 30 open/close cycles,
then 100 s of nothing.

| State | Core process | Threads | Handles | Web views |
|---|---|---|---|---|
| Idle, just started | 15.3 MB (4.9 private) | 2 | 154 | none |
| Round open, nothing picked | 33.8 MB (10.9 private) | 29 | 402 | 8 processes, 427 MB |
| After 200 clicks | 71.5 MB (45.5 private) | 30 | 438 | 8 processes, 571 MB |
| Round closed, bar warm | 39.6 MB (13.5 private) | 27 | 389 | 6 processes, 342 MB |
| After 30 open/close cycles | 41.1 MB (16.8 private) | 56 | 418 | 6 processes, 346 MB |
| Idle again, bar let go | 40.2 MB (15.0 private) | 24 | 344 | none |

- 200 clicks in 17.9 s gave 199 picks, all saved; none reached the page. The one missing click
  landed on Clipframes' own comment box, which had opened under the pointer.
- notes.md for 199 picks: 37 KB. The clipboard held all of them.
- Memory used by a long round is given back when it closes; threads and handles return to
  where they were after 30 cycles, so nothing leaks per round.
- Idle CPU afterwards: 0.016 s in 30 s (0.05% of one core).

Still to run: the 50,000-element page (`demo/stress/big.html`), several screens, 8 h idle,
and everything on macOS as a whole app.

## 2026-10-09: cold open after reordering, and shipped sizes

Three cold opens on fleet-win, each from a fresh start, traced with `CLIPFRAMES_TRACE=1`.

| | Before | After |
|---|---|---|
| Clicks are captured | 500–1535 ms | 35 ms |
| Bar drawn | 600–970 ms | 570–600 ms |
| All windows up | 780–1980 ms | 890–970 ms |

What changed: input starts before any window is built, and the bar is created visible and in
place instead of being shown in a second step. The 470–540 ms that remain before the bar is on
screen are WebView2 starting; nothing of ours runs in that time.

Sizes with settings, autostart and the updater included:

| | Download | Installed |
|---|---|---|
| Windows installer | 1.9 MB | 5.1 MB |
| macOS, Apple silicon and Intel in one | 4.8 MB | 10.1 MB |

Idle after an update restart on Windows: 16.5 MB, no window, no web view.

## 2026-10-09: the full app (screenshots, clips, history, places, React interface)

Same machine and method as the stress run above, after every tool was added. Each element
pick now also saves a picture of the element.

| State | Core process | Threads | Handles | Web views |
|---|---|---|---|---|
| Idle, just started | 25.3 MB (7.5 private) | 8 | 263 | none |
| Round open, nothing picked | 39.2 MB (12.8 private) | 32 | 447 | 8 processes, 431 MB |
| After 200 clicks | 103.5 MB (48.1 private) | 32 | 450 | 8 processes, 596 MB |
| Round closed, bar warm | 41.1 MB (14.5 private) | 29 | 403 | 6 processes, 365 MB |
| After 30 open/close cycles | 42.7 MB (18.1 private) | 58 | 435 | 6 processes, 362 MB |
| Idle again, bar let go | 41.8 MB (16.2 private) | 26 | 361 | none |

- 200 clicks in 17.8 s gave 200 picks and 200 pictures (40 MB on disk); none reached the page.
- Idle with the once-a-second look at the front window and the tab on screen, over 60 s:
  37.3 MB (11.0 private), no web view, 0.000 s of CPU. The tab is a native window.
- Pictures show only the app underneath: Clipframes' own windows are excluded from capture.
- A 5.5 s clip of a 1900 × 300 px area: 22 frames at 1600 px wide, 1.3 MB.

History with 5,000 captures (`cargo run --release --example cf-history`), first page of 40:

| | First run | Median of 5 |
|---|---|---|
| macOS (hub) | 4 ms | 3 ms |
| Windows (fleet-win) | 13 ms | 8 ms |

The first version checked every folder for its capture file and took 402 ms on Windows; it now
lists the folder by name and reads only the page shown. Budget: 300 ms.

Shipped interface: 77 KB of script and 2 KB of styles, gzipped (React 19 and Zustand included).

## 2026-10-09, night: after the interface was simplified and reporting was added

Windows 11 on fleet-win, same method as the stress run above (`tools/win-stress.ps1`), this
time with Chrome on the left 60% of the screen and a terminal on the right.

| State | Core process | Threads | Handles | Web views |
|---|---|---|---|---|
| Idle, just started | 25.3 MB (9.5 private) | 7 | 261 | none |
| Round open, nothing picked | 39.0 MB (12.7 private) | 31 | 445 | 8 processes, 432 MB |
| After 200 clicks | 102.7 MB (46.5 private) | 31 | 448 | 8 processes, 559 MB |
| Round closed, bar warm | 71.0 MB (15.3 private) | 29 | 403 | 6 processes, 365 MB |
| After 30 open/close cycles | 43.1 MB (17.4 private) | 58 | 435 | 6 processes, 359 MB |
| 100 s later | 42.5 MB (17.8 private) | 47 | 422 | 6 processes, 362 MB |
| 4 minutes later | 42 MB | 26 | | none, 0.000 s CPU in 20 s |

- 200 clicks in 18.1 s gave 200 picks, all saved. notes.md 49 KB.
- "Idle, just started" is 10 MB higher than before. The start report goes out in the first
  seconds, which loads the HTTP and encryption code and starts the runtime's threads. An
  installed copy loads the same code 30 s after start for its update check, so the earlier
  15 MB figure was only true for a copy that never checked for updates.
- At the 100 s mark six web view processes were still there and the core had used 0.9 s of
  CPU in 30 s; four minutes later there were none and CPU use was zero. The earlier run had
  none at 100 s. Not explained; the keep-warm time is 90 s, so the margin is small. Worth a
  closer look.
- The run remembered Windows Terminal as a place (clicks landed on it), so the tab then
  showed over the terminal. That is the feature working, but a stress run should clean
  `places.json` afterwards.

Updates, both systems, against a local feed (a 0.3.0 build whose endpoint is
`http://127.0.0.1:5991/latest.json`, and a signed 0.3.1):

- Windows: started hidden, asked the feed after 30 s, downloaded the installer, installed and
  restarted hidden as 0.3.1, then asked the feed again and stayed. About 35 s in all.
- macOS 27 (Apple silicon, fleet-air): the same, with the app bundle replaced in place.

macOS, without a screen (fleet-air, over ssh; Accessibility not granted, so nothing was picked):

- Starts hidden: 77 to 78 MB resident, 0.0% CPU. Bar drawn 166 to 275 ms after it is asked for.
- With the permission missing the round does not start and says so.
- The whole flow with real pointer events is ready to run there: build with
  `--features selftest`, start with `--selftest`.

Idle for ninety minutes on Windows (commit 3ac1e53, started hidden, the demo site in front
so the tab was on screen, nobody touching the machine; one reading a minute, 02:40 to 04:09):

- Memory: 22.1 to 25.8 MB working set, 7.0 to 7.2 MB private. It ended lower than it started.
- CPU: 0.39 s of processor time in 89 minutes, which is 0.007% of one core.
- Threads 5 to 10, handles 265 to 270, no web view at any reading.
- Nothing grew over the run. The eight hour run is still to do.
