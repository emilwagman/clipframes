# Clipframes, the desktop app

Point at the thing you want changed, and Clipframes tells your agent what it is. One app for
macOS and Windows: a Rust core (Tauri) and a React interface.

- `ARCHITECTURE.md`: how the parts fit and why.
- `PERFORMANCE.md`: what was measured, on which machine, and how to repeat it.

## Working on it

```
pnpm install
pnpm tauri dev                                   # the app, reloading as you edit
cd src-tauri && cargo test --lib                 # the core's tests
pnpm exec vite --port 5321                       # then open /lab.html: the interface over a demo page, no app needed
cd src-tauri && cargo build --release            # the real app, interface built in (run `pnpm build` first)
```

Tools, built on request (`cargo build --release --example <name>`): `cf-probe` prints the element
under the pointer, `cf-pick` runs a picking round in the terminal, `cf-bench` times element
readings, `cf-history` times History with thousands of captures.

`CLIPFRAMES_TRACE=1` prints the order of opens, picks and closes. `CLIPFRAMES_CAPTURABLE=1`
lets screen recorders see Clipframes' own windows, for recording a demo of it.

## Releasing

`./release.sh "What changed"` builds, signs and notarizes both systems into `../dist/<version>/`.
Add `--publish` to create the GitHub release that installed copies update from.
