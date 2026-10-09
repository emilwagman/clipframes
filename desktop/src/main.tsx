// Every Clipframes window loads this page; the window's name says which part it is.

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

// These are tool windows, not pages: no context menu.
window.addEventListener("contextmenu", (event) => event.preventDefault());

const view = label === "bar" ? <Bar /> : label === "note" ? <Note /> : label === "settings" ? <Settings /> : label === "history" ? <History /> : label === "tab" ? <TabMark /> : <Overlay />;
createRoot(document.getElementById("root") as HTMLElement).render(view);
