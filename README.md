# Clipframes

Clipframes is a macOS menu bar app for showing a coding agent what you mean. You point at something on screen, and Clipframes copies a reference you can paste into Claude Code or Codex.

It has three tools:

- **Element** (⌃⇧1): click one thing, like a button or a card. Clipframes saves a screenshot and reads the element's name, role and, in browsers and Electron apps, its DOM id, classes and URL.
- **Screenshot** (⌃⇧2): drag an area. Clipframes saves the image and names the elements inside it.
- **Clip** (⌃⇧3, press again to stop): record an area. Clipframes saves the video, a set of frames and a timeline of what you clicked.

Each capture is a folder in `~/Clipframes` with a `notes.md` that the agent reads. Captures stay on your Mac, and the app goes online only to check GitHub for updates. You can turn the update check off in Settings.

## Install

Download the latest zip from [Releases](https://github.com/emilwagman/clipframes/releases/latest), unzip it and move Clipframes to Applications. It needs macOS 15 or later and runs on Apple Silicon and Intel Macs.

On first launch it asks for two permissions in System Settings: Screen Recording, to take screenshots and clips, and Accessibility, to read which element you pointed at.

Clipframes updates itself through [Sparkle](https://sparkle-project.org).

## Build

```
app/build.sh               # builds and installs to ~/Applications
NO_INSTALL=1 app/build.sh  # builds to app/build/ only
```

The build uses `swiftc` directly, without an Xcode project. It downloads Sparkle into `app/vendor/` the first time.

## Release

1. Bump `CFBundleShortVersionString` and `CFBundleVersion` in `app/Info.plist` and commit.
2. Run `app/release.sh "One line per change"`.

The script builds, notarizes, publishes a GitHub release and adds the version to `appcast.xml`, which installed copies check for updates. The comment at the top of the script lists what it needs on the Mac it runs on.
