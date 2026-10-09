// Every Clipframes window loads this page; the window's name says which part it is.

import "../ui/style.css";
import { mountBar } from "../ui/bar";
import { mountNote } from "../ui/note";
import { mountOverlay } from "../ui/overlay";
import { label, platform } from "./tauri";

const root = document.body;
if (label === "bar") mountBar(root, platform);
else if (label === "note") mountNote(root, platform);
else mountOverlay(root, platform);

// These are tool windows, not pages: no context menu.
window.addEventListener("contextmenu", (event) => event.preventDefault());
