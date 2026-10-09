// The desktop app's side of the door: commands and events from the Rust core.

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { HoverView, MarkView, Platform, RoundView } from "../ui/platform";

const here = getCurrentWebviewWindow();
const mac = navigator.userAgent.includes("Mac");

function on<T>(event: string, listener: (payload: T) => void): void {
  void here.listen<T>(event, (e) => listener(e.payload));
}

export const label = here.label;

export const platform: Platform = {
  shortcut: mac ? "⌃⇧Space" : "Ctrl+Shift+Space",
  state: () => invoke<RoundView>("round_state"),
  onRound: (listener) => on<RoundView>("round", listener),
  onHover: (listener) => on<HoverView>("hover", listener),
  onMarks: (listener) => on<MarkView[]>("marks", listener),
  setNote: (index, note) => invoke("note_set", { index, note }),
  closeNote: () => invoke("note_close"),
  removePick: (index) => invoke("pick_remove", { index }),
  done: () => invoke("round_done"),
  escape: () => invoke("escape_key"),
  openPermission: () => invoke("permission_open"),
};
