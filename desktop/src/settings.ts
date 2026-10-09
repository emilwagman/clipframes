// Settings: the shortcut, starting at login, and the version. Only the desktop app has this
// window, so it talks to the core directly instead of through the shared Platform.

import { invoke } from "@tauri-apps/api/core";
import { el } from "../ui/dom";
import { listen } from "./tauri";

interface SettingsView {
  shortcut: string;
  shortcutLabel: string;
  shortcutWorks: boolean;
  launchAtLogin: boolean;
  version: string;
  update: string;
  mac: boolean;
}

/** "ctrl+shift+Space" from a key press, or null while only modifiers are down. */
function combo(event: KeyboardEvent): string | null {
  if (["Control", "Shift", "Alt", "Meta", "OS"].includes(event.key)) return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("ctrl");
  if (event.altKey) parts.push("alt");
  if (event.shiftKey) parts.push("shift");
  if (event.metaKey) parts.push("super");
  parts.push(event.code);
  return parts.join("+");
}

export function mountSettings(root: HTMLElement): void {
  document.documentElement.classList.add("window");
  const key = el("button", { class: "key" });
  const keyHelp = el("p", { class: "help" });
  const login = el("input", { type: "checkbox", id: "login" });
  const version = el("p", { class: "help" });
  const check = el("button", { class: "plain", text: "Check for updates" });

  root.append(
    el(
      "main",
      { class: "settings" },
      el("h1", { text: "Clipframes" }),
      el("section", {}, el("h2", { text: "Shortcut" }), el("div", { class: "row" }, el("span", { text: "Open and close the bar" }), key), keyHelp),
      el("section", {}, el("label", { class: "row", for: "login" }, el("span", { text: "Start when I log in" }), login), el("p", { class: "help", text: "Starts with nothing on screen, ready for the shortcut." })),
      el("section", {}, el("h2", { text: "Updates" }), el("div", { class: "row" }, version, check), el("p", { class: "help", text: "New versions install themselves." })),
    ),
  );

  let current: SettingsView | null = null;
  let recording = false;
  let message = "";

  const draw = (view: SettingsView) => {
    current = view;
    login.checked = view.launchAtLogin;
    version.textContent = view.update || `Version ${view.version}`;
    key.classList.toggle("recording", recording);
    key.textContent = recording ? "Press the new shortcut" : view.shortcutLabel;
    keyHelp.textContent = recording
      ? "Esc to keep the current one."
      : message || (view.shortcutWorks ? "Click it to change." : `${view.shortcutLabel} is used by another app, so it does nothing here. Click it and choose another.`);
    keyHelp.classList.toggle("warn", !recording && (message !== "" || !view.shortcutWorks));
  };

  key.addEventListener("click", () => {
    recording = !recording;
    message = "";
    if (current) draw(current);
  });

  window.addEventListener("keydown", (event) => {
    if (!recording) return;
    event.preventDefault();
    if (event.key === "Escape") {
      recording = false;
      if (current) draw(current);
      return;
    }
    const shortcut = combo(event);
    if (shortcut === null) return;
    recording = false;
    invoke<SettingsView>("shortcut_set", { shortcut }).then(
      (view) => {
        message = "";
        draw(view);
      },
      (error) => {
        message = String(error);
        if (current) draw(current);
      },
    );
  });

  login.addEventListener("change", () => void invoke<SettingsView>("launch_set", { on: login.checked }).then(draw));
  check.addEventListener("click", () => void invoke("update_check"));

  listen<SettingsView>("settings", draw);
  void invoke<SettingsView>("settings_get").then(draw);
}
