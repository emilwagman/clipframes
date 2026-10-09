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
