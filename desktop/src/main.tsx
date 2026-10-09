// Every Clipframes window loads this page; the window's name says which part it is.

import { invoke } from "@tauri-apps/api/core";
import { createRoot } from "react-dom/client";
import "../ui/style.css";
import { Bar } from "../ui/Bar";
import { Note } from "../ui/Note";
import { Overlay } from "../ui/Overlay";
import { connect } from "../ui/store";
import { History } from "./History";
import { Settings } from "./Settings";
import { label, platform } from "./tauri";
import { TabMark } from "./TabMark";

const own = label === "settings" || label === "history" || label === "tab";
if (!own) {
  connect(platform);
  // While another app has the keyboard, the core hears Esc itself. While one of these windows
  // has it (typing a comment), the system gives the key to the window instead, so pass it on.
  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape") void platform.escape();
  });
}

// An error in a window's own code is reported, without anything from the page it was drawn over.
const report = (message: string, at: string) => void invoke("ui_error", { message: message.slice(0, 300), at }).catch(() => {});
window.addEventListener("error", (event) => report(event.message, `${event.filename}:${event.lineno}`));
window.addEventListener("unhandledrejection", (event) => report(String(event.reason), "promise"));

// These are tool windows, not pages: no context menu.
window.addEventListener("contextmenu", (event) => event.preventDefault());

const view = label === "bar" ? <Bar /> : label === "note" ? <Note /> : label === "settings" ? <Settings /> : label === "history" ? <History /> : label === "tab" ? <TabMark /> : <Overlay />;
createRoot(document.getElementById("root") as HTMLElement).render(view);
