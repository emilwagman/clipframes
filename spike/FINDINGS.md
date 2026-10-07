# Accessibility tree spike, 2026-09-29

Run on the MacBook Air (macOS 15.7.5) over ssh with `axspike` (source here).
`sshd-keygen-wrapper` already had Accessibility there, so no settings were touched.
A 4×4 or 5×5 grid of hit-tests per front window simulates "pin anywhere";
a tree walk measures what the app exposes.

| App | Kind | What a pin gets | Verdict |
|---|---|---|---|
| Chrome, localhost page | Browser | role + accessible name + DOM id + class list + URL; unnamed divs give inner text (`Group .btn text: Delete workspace`) | Excellent |
| Safari, same page | Browser (WebKit) | same as Chrome, plus Safari's own chrome has ids | Excellent |
| Incredible | Tauri / WKWebView | `Button "Continue with Google" .brandauth-btn--google`, URL `tauri://localhost`; exposed without any wake | Excellent |
| Slack | Electron | after `AXManualAccessibility`: names + BEM classes (`.p-zoom__button--zoom_in`) | Excellent after wake |
| Cursor | Electron (VS Code) | after wake: `Button "Open Cursor Settings" .codicon-settings-gear` | Excellent after wake, but see side effect |
| System Settings | SwiftUI | labels + ids (`OpaqueProviderGroup id=resolutionSection`) | Good |
| Notes | AppKit | ids like `Note Body Text View`, `_NS:50` (auto ids, not useful) | Good for labels |
| Calendar | AppKit, custom-drawn grid | hit-test returns only the window | Poor; needs OCR |
| Ghostty | Terminal | one `TextArea "Terminal content area"` for the whole pane | Poor; needs text at position or OCR |

Not tested: Figma (not installed), iOS Simulator (not installed).

## Findings

1. **Web content is the sweet spot.** Browsers, Tauri and Electron all give DOM
   id + class list + accessible name + URL. That is close to agentation's
   selectors and is searchable in a codebase. What's missing vs agentation:
   React component names and custom `data-*` attributes (`data-component`,
   `data-testid`) are not in the tree.
2. **Chromium/Electron need a wake flag.** `AXManualAccessibility=true`
   worked for Slack and Cursor (tree appeared after ~6 s; 1 s was too short).
   Chrome was already exposing its tree on the Air (other assistive apps are
   running there), so its wake path is unverified on a clean machine.
3. **Side effect:** waking Cursor made it show "Screen reader usage detected.
   Do you want to enable screen reader optimized mode?". VS Code-based apps
   will do this. Wake only while the inspector or a recording is active, turn it
   off afterwards (`unwake`), and expect the prompt once per launch anyway.
4. **Hit-testing is fast:** first lookup 12–30 ms, then 0–2 ms. Fine for live
   hover at 10+ Hz.
5. **Hit-tests land on the deepest element,** often an unnamed Group. Walking
   up to the nearest named ancestor, and collecting inner text, fixes almost
   all of these.
6. **Custom-drawn UIs and terminals need OCR** (Vision) around the pin as the
   fallback. That covers Calendar, Ghostty, canvas charts, and presumably Figma.
7. **Not tested:** the click timeline during recording (global mouse monitor),
   which uses the same hit-test and should behave like pins.
