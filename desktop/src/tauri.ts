// The desktop app's side of the door: commands and events from the Rust core.

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { AreaView, HoverView, MarkView, Platform, RoundView } from "../ui/platform";

const here = getCurrentWebviewWindow();

/** Listens to an event from the core, sent to this window or to all of them. */
export function listen<T>(event: string, listener: (payload: T) => void): void {
  void here.listen<T>(event, (e) => listener(e.payload));
}

export const label = here.label;

// Comment text goes to the core one message per keystroke. Each carries a number counted up
// from one, so the core can drop one that arrives late. Every round has a new comment box
// (this script, loaded again), and the core starts over with it.
let noteSeq = 0;

export const platform: Platform = {
  tools: ["element", "area", "clip"],
  history: true,
  state: () => invoke<RoundView>("round_state"),
  onRound: (listener) => listen<RoundView>("round", listener),
  onHover: (listener) => listen<HoverView>("hover", listener),
  onMarks: (listener) => listen<MarkView[]>("marks", listener),
  onArea: (listener) => listen<AreaView>("area", listener),
  onFaint: (listener) => listen<boolean>("faint", listener),
  setTool: (tool) => invoke("tool_set", { tool }),
  stopRecording: () => invoke("recording_stop"),
  setNote: (index, note) => invoke("note_set", { index, note, seq: ++noteSeq }),
  closeNote: () => invoke("note_close"),
  resizeNote: (height) => invoke("note_resize", { height }),
  removePick: (index) => invoke("pick_remove", { index }),
  setAuto: (on) => invoke("place_auto_set", { on }),
  moveBar: (home) => invoke(home ? "bar_home" : "bar_grip"),
  openHistory: () => invoke("history_open"),
  done: () => invoke("round_done"),
  escape: () => invoke("escape_key"),
  openPermission: (kind) => invoke("permission_open", { kind }),
  quit: () => invoke("app_quit"),
};
