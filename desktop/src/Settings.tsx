// Settings: the shortcut, starting at login, and the version. Only the desktop app has this
// window, so it talks to the core directly instead of through the shared Platform.

import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { listen } from "./tauri";

interface SettingsView {
  shortcut: string;
  shortcutLabel: string;
  shortcutWorks: boolean;
  launchAtLogin: boolean;
  shareUsage: boolean;
  usageAvailable: boolean;
  version: string;
  update: string;
  mac: boolean;
}

/** "ctrl+shift+Space" from a key press, or null while only modifiers are down. */
function combo(event: KeyboardEvent): string | null {
  // Ctrl+Alt arrives as "AltGraph" on Windows; some keyboards and tools send no code at all.
  if (["Control", "Shift", "Alt", "AltGraph", "Meta", "OS"].includes(event.key)) return null;
  if (event.code === "" || /^(Control|Shift|Alt|Meta|OS)(Left|Right)$/.test(event.code)) return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("ctrl");
  if (event.altKey) parts.push("alt");
  if (event.shiftKey) parts.push("shift");
  if (event.metaKey) parts.push("super");
  parts.push(event.code);
  return parts.join("+");
}

export function Settings() {
  const [view, setView] = useState<SettingsView | null>(null);
  const [recording, setRecording] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    document.documentElement.classList.add("window");
    listen<SettingsView>("settings", setView);
    void invoke<SettingsView>("settings_get").then(setView);
  }, []);

  useEffect(() => {
    if (!recording) return;
    const onKey = (event: KeyboardEvent) => {
      event.preventDefault();
      if (event.key === "Escape") return setRecording(false);
      const shortcut = combo(event);
      if (shortcut === null) return;
      setRecording(false);
      invoke<SettingsView>("shortcut_set", { shortcut }).then(
        (next) => {
          setMessage("");
          setView(next);
        },
        (error) => setMessage(String(error)),
      );
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [recording]);

  if (!view) return null;
  const warn = !recording && (message !== "" || !view.shortcutWorks);
  const help = recording
    ? "Esc keeps the current one."
    : message || (view.shortcutWorks ? "Click it to change." : `${view.shortcutLabel} is used by another app, so it does nothing here. Click it and choose another.`);

  return (
    <main className="settings">
      <h1>Clipframes</h1>
      <section>
        <h2>Shortcut</h2>
        <div className="row">
          <span>Open and close the bar</span>
          <button className={recording ? "key recording" : "key"} onClick={() => (setMessage(""), setRecording(!recording))}>
            {recording ? "Press the new shortcut" : view.shortcutLabel}
          </button>
        </div>
        <p className={warn ? "help warn" : "help"}>{help}</p>
      </section>
      <section>
        <label className="row" htmlFor="login">
          <span>Start when I log in</span>
          <input id="login" type="checkbox" checked={view.launchAtLogin} onChange={(e) => void invoke<SettingsView>("launch_set", { on: e.target.checked }).then(setView)} />
        </label>
        <p className="help">Starts with nothing on screen, ready for the shortcut.</p>
      </section>
      {view.usageAvailable && (
        <section>
          <label className="row" htmlFor="usage">
            <span>Share anonymous usage</span>
            <input id="usage" type="checkbox" checked={view.shareUsage} onChange={(e) => void invoke<SettingsView>("usage_set", { on: e.target.checked }).then(setView)} />
          </label>
          <p className="help">Counts of what you use and reports of errors, to find and fix problems. Never what is on your screen or what you write.</p>
        </section>
      )}
      <section>
        <h2>Updates</h2>
        <div className="row">
          <p className="help">{view.update || `Version ${view.version}`}</p>
          <button className="plain" onClick={() => void invoke("update_check")}>
            Check for updates
          </button>
        </div>
        <p className="help">New versions install themselves.</p>
      </section>
    </main>
  );
}
