// Every Clipframes window loads this page; the window's name says which part it is.

import "../ui/style.css";
import { mountBar } from "../ui/bar";
import { mountNote } from "../ui/note";
import { mountOverlay } from "../ui/overlay";
import { mountSettings } from "./settings";
import { label, platform } from "./tauri";

const root = document.body;
if (label === "bar") mountBar(root, platform);
else if (label === "note") mountNote(root, platform);
else if (label === "settings") mountSettings(root);
else mountOverlay(root, platform);

// While another app has the keyboard, the core hears Esc itself. While one of these windows
// has it (typing a comment), the system gives the key to the window instead, so pass it on.
if (label !== "settings") {
  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape") void platform.escape();
  });
}

// These are tool windows, not pages: no context menu.
window.addEventListener("contextmenu", (event) => event.preventDefault());
