# Clipframes desktop: how it is built

One app for macOS, Windows and Linux, built with Tauri 2: a Rust core and a web interface.
It replaces the Swift app in `../app`, which stays only as a reference until this one can do
everything it does.

## What it must be

- **Tiny.** A small download, little memory, no work while idle. It runs all day in the
  background, so every dependency has to earn its place. Budgets are in "Budgets" below.
- **Never laggy.** Nothing that asks the operating system a question runs on the interface's
  thread. The interface only draws.
- **The same on every system.** The interface and the capture format are written once. Only
  the four operating-system jobs differ, each behind one small Rust interface.
- **Present where you use it.** After a capture in an app or on a site, a small tab appears
  the next time that app or site is open.
- **Collect, then copy.** Every pick updates the clipboard with everything picked so far, so
  one change and five changes are the same flow.

## The parts

```
desktop/
  ui/          the interface, in React with one Zustand store: the bar, the tab, the
               overlay painter, the comment box, the library. It talks to the outside only
               through `Platform` (ui/platform.ts).
  src/         one small entry file per window, each mounting something from ui/ with the
               Tauri platform.
  src-tauri/   the Rust core.
../site/       the website mounts the same ui/ with a web platform that reads the page's own
               elements, so visitors use the real bar on the site.
```

### The Rust core (`src-tauri/src`)

| Module | Job | macOS | Windows | Linux |
|---|---|---|---|---|
| `element` | what is under a point, in another app | Accessibility API | UI Automation | AT-SPI (not built) |
| `picker` | follow the pointer and take the click while picking | event tap | low-level mouse hook | X11 first; Wayland limited |
| `shot` | picture of an area, later a recording | ScreenCaptureKit | Windows Graphics Capture | portal |
| `places` | which app and site is in front, and where Clipframes was used | front app + page URL | foreground window + page URL | later |
| `store` | captures on disk: folders, capture.json, notes.md, the reference line | shared | shared | shared |

`element` and `picker` are the pieces that had to be proven first. `cf-probe` and `cf-bench`
(src/bin) check them on any machine.

### Picking, step by step

1. The shortcut (or a click on the tab) starts a round. The overlay windows appear, one per
   screen: transparent, on top, and **click-through**, so they only paint.
2. `picker` watches the pointer. On each move it hands the newest position to one worker
   thread, which asks `element` what is there. Older positions that were not read yet are
   dropped: only the latest matters. The answer goes to the overlay as an event.
3. The overlay draws the highlight and the name tag. It never waits for anything.
4. A click is taken by `picker` (it does not reach the app underneath) and becomes a pick. The
   comment box opens beside the element; a note is optional.
5. Every pick rewrites the clipboard with all picks of the round, and the bar shows the count.
6. Esc, the shortcut again, or Done ends the round and saves it as one capture.

Because the overlay is click-through, asking "what is at this point" can never return
Clipframes itself. On macOS the reader also skips Clipframes' own windows explicitly.

### The interface and `Platform`

`ui/platform.ts` is the only door between the interface and the world:

- start and stop a picking round, and receive hover and pick events
- write the clipboard
- read and write captures and settings
- know which place (app or site) is in front

Two implementations: `src/tauri-platform.ts` (the app) and `site/…/web-platform.ts` (the
website: elements come from the page's DOM, captures stay in memory). The components do not
know which one they have.

## Budgets

Measured at every milestone, on macOS and Windows, and recorded in `PERFORMANCE.md`.

| What | Budget |
|---|---|
| Download | under 15 MB |
| Memory, idle in the background | under 100 MB across all its processes |
| CPU, idle | under 0.5% of one core, averaged over a minute |
| Reading the element under the pointer | median under 5 ms, 95th percentile under 16 ms |
| Pointer move to highlight on screen | within 2 frames |
| Library with 5,000 captures | opens in under 300 ms, scrolls without dropped frames |

Stress cases to pass before a release: a page with 50,000 elements, 30 windows open, three
screens, 200 picks in one round, a 10-minute recording, the pointer moving at full speed for
a minute, the app left idle for 8 hours (memory must not grow).

## Idle size against opening speed

Measured on Windows (PERFORMANCE.md): a web view, even hidden, holds a few hundred megabytes,
and starting one takes most of a second. So:

- While idle there is no web view. The core alone is about 15 MB and uses no CPU.
- Opening the bar starts input first, then builds the windows. After closing, the bar stays
  hidden for 90 s so the next open is instant; then it is destroyed.
- Anything that is on screen for long stretches must not be a web view. The small tab that
  appears when a remembered app or site is opened is that kind of thing: it will be a native
  window per system (a layered window on Windows, a panel on macOS), drawn by the core.
- Still open: the cold open takes 0.6 to 1.0 s on Windows. Choices are a longer keep-warm
  after real use, a native bar, or accepting it.
