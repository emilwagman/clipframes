# What Clipframes reports

Clipframes sends anonymous counts of what is used and reports of errors, so problems can be
found and fixed. It can be turned off in Settings ("Share anonymous usage").

It never sends what is on your screen, the names or text of what you pick, your comments,
pictures, file paths, window titles, app names or site addresses.

Every event carries: a random install number made on first run (it identifies a copy of the
app, not a person), the app version, the operating system (`macos`, `windows`, `linux`) and
the processor type, and the sender's name (`clipframes`). No person profile is made and no
location is looked up from the address the event came from.

Events wait up to ten seconds and leave in small batches. Turning the setting off also drops
the ones still waiting.

| Event | When | What is sent with it |
|---|---|---|
| `app_started` | The app starts | how (`login`, `hand`, `update`), first run or not, whether the shortcut could be taken, whether start at login is on |
| `round_opened` | The bar opens | how (`shortcut`, `tab`, `tray`, `launch`, `other`), whether the bar was kept warm, milliseconds until clicks were captured and until all windows were up, number of displays |
| `round_blocked` | The bar opens but cannot pick | how it was asked for (as for `round_opened`), why (`permission`, `picker`) |
| `pick_added` | Something is picked | kind (`element`, `area`, `clip`), whether a picture was taken; for an element whether it had a selector, a name and a web address (yes or no each); for an area its size in pixels; for a clip its seconds, frames and number of clicks |
| `pick_removed` | Remove is pressed | nothing |
| `round_closed` | The bar closes | number of picks of each kind, how many had a comment, seconds the bar was open |
| `history_copied` | Copy is pressed in History | number of picks copied |
| `history_deleted` | A capture is deleted in History | nothing |
| `place_auto_set` | The pin in the bar is pressed | on or off |
| `shortcut_changed` | A new shortcut is saved | nothing |
| `claude_read_set` | "Let Claude Code read captures without asking" is switched, and Claude Code's settings file was changed | on or off |
| `update_found` | A new version is found | its version number |
| `$exception` | Something failed: a screenshot, an update, the picker, a crash, or an error in a window's own code | the kind (`capture`, `update`, `picker`, `panic`, `ui`), a message, and the place in Clipframes' code (a function, or a file name and line; for a window also which window) |

The message of an `$exception` is the error's text up to the first thing in it that looks like
a file path or an address, and at most 300 characters. For an update it is only the name of
the kind of error (`Minisign`, `Io`, …), never its text. An update check that failed because
the computer was offline, or because no release exists yet, is not reported at all.

The code is `src-tauri/src/telemetry.rs`. A build without `CLIPFRAMES_POSTHOG_KEY` set at
build time has nowhere to send to, sends nothing, and does not show the setting.
