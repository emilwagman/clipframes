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
readings, `cf-history` times History with thousands of captures, `cf-read-rule` prints the rule
the "Let Claude Code read captures without asking" switch adds.

That switch changes Claude Code's settings file (`src-tauri/src/claude.rs`). To try it without
touching your own, start the app with `CLAUDE_CONFIG_DIR` set to a folder holding a copy of
`settings.json`, or give `cf-read-rule <folder> on <file>` a file of your choosing. The rule was
checked against real Claude Code sessions on macOS only. On Windows, check it once by hand:

```
mkdir %TEMP%\cf-rule && echo {} > %TEMP%\cf-rule\settings.json
cf-read-rule %USERPROFILE%\Clipframes on %TEMP%\cf-rule\settings.json
claude -p "Read <a notes.md in %USERPROFILE%\Clipframes> and say its first line" --settings %TEMP%\cf-rule\settings.json
```

The read must go through without a refusal, and be refused when `--settings` is left out. Then the
same with a captures folder on another drive, which gives a rule of the form `Read(//d/Captures/**)`.

`CLIPFRAMES_TRACE=1` prints the order of opens, picks and closes. `CLIPFRAMES_CAPTURABLE=1`
lets screen recorders see Clipframes' own windows, for recording a demo of it.

## Releasing

`./release.sh "What changed"` builds, signs and notarizes both systems into `../dist/<version>/`.
Add `--publish` to create the GitHub release that installed copies update from.
