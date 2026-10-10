# Clipframes

Clipframes is a small app for Mac and Windows for showing a coding agent what you mean. You point at something on screen, write what should change, and paste the result into Claude Code, Codex or any other agent.

It works in every app on your computer: a web app in a browser, desktop apps built with Electron or Tauri, and native apps. You don't add anything to your project.

## How it works

Press Ctrl+Shift+Space (⌃⇧Space on a Mac) from any app. A small bar appears, and whatever is under the pointer is outlined and named.

- **Element:** click one thing, like a button or a card. Clipframes reads its name and role and, in browsers and Electron apps, its id and classes, and saves a picture of it.
- **Area:** drag over part of the screen to send a picture of it.
- **Clip:** drag over an area and use the app as usual while it records. Clipframes saves the frames in order and a list of what you clicked.

After each pick a small box asks what should change. Pick as many things as you like; the clipboard is up to date after every one. Then paste once:

```
[Clipframes: 2 things in Google Chrome "Invoices". Read ~/Clipframes/2026-10-09_11-42-30/notes.md]
1. Button "New invoice" (#new-invoice .btn.btn-primary), under heading "Invoices": make this green
2. DataItem "Paid", 2nd of 4 on the page, under heading "Invoices": make these grey
```

When the same thing appears more than once, the line says which one you picked and the heading it sits under, so the agent does not change all of them.

Each round is a folder in `~/Clipframes` with a `notes.md` the agent reads, and the pictures. History lists past rounds so you can copy one again.

Clipframes remembers the apps and sites you used it on and shows a small tab there next time. A pin in the bar turns that off for a place.

## Claude Code and the captures folder

Claude Code asks before it reads a file outside the project it is working in, and captures are in `~/Clipframes`. So the first thing that happens after a paste is a question. Settings has a switch for this: "Let Claude Code read captures without asking". It is off until you switch it on.

Switched on, Clipframes adds one line to the `allow` list in Claude Code's own settings file (`~/.claude/settings.json`, on Windows `%USERPROFILE%\.claude\settings.json`):

```
"Read(~/Clipframes/**)"
```

Switched off, it removes that line and nothing else. The rest of the file is left as it was, and a copy of the file from before the first change is kept beside it as `settings.json.clipframes-backup-<date>`. If Clipframes cannot read the file as plain JSON it does not change it, and shows the line so you can add it yourself. If Claude Code has not made the file yet, Clipframes makes it with only this line in it. The switch shows what is in the file, so adding or removing the line by hand moves it too.

If `~/Clipframes` is a link to a folder somewhere else, a second line for that folder is added, because Claude Code checks both.

Codex needs nothing like this: it reads files outside the project without asking.

## Install

Download the latest version from [Releases](https://github.com/emilwagman/clipframes/releases/latest).

- **Mac:** unzip and move Clipframes to Applications. On first use it asks for two permissions in System Settings: Accessibility, to read which element you pointed at, and Screen Recording, to take pictures and clips.
- **Windows:** run the installer. It needs no permissions.

Clipframes starts with the computer, with nothing on screen, and updates itself.

## What leaves your computer

Your captures stay on your computer. Clipframes goes online for two things:

- It checks GitHub for a new version.
- It sends anonymous counts of what is used and reports of errors, so problems can be found and fixed. This never includes what is on your screen, what you picked or what you wrote. [desktop/TELEMETRY.md](desktop/TELEMETRY.md) lists every event, and Settings has a switch to turn it off.

## Working on it

The app is in [`desktop/`](desktop/README.md): a Rust core (Tauri) and a React interface. The website is in `site/`.
