// The real bar, comment box and overlay over the demo page, in one browser window, so the look
// can be judged without running the app. ?state=empty|picked|area|recording picks the moment.
// ?note=box|line|ghost|dock shows each way of writing a comment that is being tried (ghost
// with &faint=1: the pointer over it), and ?said= puts words in it.
// ?page=dark shows it over a dark page, and ?tab=1 adds the tab beside the bar.
import { createRoot } from "react-dom/client";
import "../ui/style.css";
import { Bar } from "../ui/Bar";
import { Note } from "../ui/Note";
import { Overlay } from "../ui/Overlay";
import type { Platform, Rect, RoundView } from "../ui/platform";
import { EMPTY_ROUND } from "../ui/platform";
import { connect, useStore } from "../ui/store";
import { History } from "../src/History";
import type { HistoryApi } from "../src/History";
import { TabMark } from "../src/TabMark";

const query = new URLSearchParams(location.search);
const state = query.get("state") ?? "picked";
// ?look= tries the interface in another colour.
if (query.get("look")) document.documentElement.dataset.look = query.get("look") as string;
const BAR = { w: 404, h: 64 };
const NOTE = { w: 316, h: 172 };
const noteStyle = query.get("note") ?? "box";

// Example captures for the History window.
const examples: HistoryApi = {
  list: async () => ({
    total: 4,
    next: 4,
    entries: [
      { id: "a", title: 'Button "New invoice" and 2 more', when: "2026-10-09 11:42", count: 3, images: ["/lab/thumbs/1.png", "/lab/thumbs/2.png", "/lab/thumbs/3.png"] },
      { id: "b", title: "Screen clip, 6 s", when: "2026-10-09 10:15", count: 1, images: ["/lab/thumbs/4.png"] },
      { id: "c", title: 'Text "$3,120" and 1 more', when: "2026-10-08 16:03", count: 2, images: ["/lab/thumbs/3.png", "/lab/thumbs/2.png"] },
      { id: "d", title: "Screenshot", when: "2026-10-08 09:27", count: 1, images: ["/lab/thumbs/2.png"] },
    ],
  }),
  copy: async () => {}, reveal: async () => {}, remove: async () => {},
  src: (path) => path,
};

const frame = document.getElementById("page") as HTMLIFrameElement;
// The demo page with its colours turned over: what a dark editor or terminal is to the bar.
if (query.get("page") === "dark") frame.style.filter = "invert(1) hue-rotate(180deg)";
const start = () => {
  const doc = frame.contentDocument as Document;
  const rect = (selector: string): Rect => {
    const r = (doc.querySelector(selector) as Element).getBoundingClientRect();
    return { x: r.x, y: r.y, width: r.width, height: r.height };
  };
  const button = rect("#new-invoice");
  const overdue = rect("#overdue-total strong");
  const table = rect("table");
  const area: Rect = { x: table.x - 8, y: table.y - 8, width: table.width * 0.62, height: 190 };

  const picked = state === "picked";
  const round: RoundView = {
    ...EMPTY_ROUND,
    picking: true,
    tool: state === "area" ? "area" : state === "recording" ? "clip" : "element",
    picks: picked
      ? [
          { kind: "element", headline: 'Button "New invoice"', selector: "#new-invoice .btn.btn-primary", note: "make this green" },
          { kind: "element", headline: 'Text "$3,120"', selector: "#overdue-total .stat.overdue", note: query.get("said") ?? "too alarming, use the normal text colour" },
        ]
      : [],
    noting: picked ? 1 : null,
    recording: state === "recording" ? 7 : null,
    // ?state=screen and ?state=screen-asked: a picture tool without Screen Recording on a Mac.
    trouble: state.startsWith("screen") ? state : null,
    shortcut: "Ctrl+Shift+Space",
    place: { name: "Google Chrome · localhost:3000", auto: true },
    noteStyle,
  };
  const none = async () => {};
  const platform: Platform = {
    tools: ["element", "area", "clip"], history: true,
    state: async () => round, onRound: none, onHover: none, onMarks: none, onArea: none,
    setTool: none, stopRecording: none, setNote: none, closeNote: none, removePick: none, setAuto: none, moveBar: none, openHistory: none, done: none, escape: none, openPermission: none, quit: none,
  };
  connect(platform);
  useStore.setState({
    round,
    hover: state === "empty" ? { rect: button, label: 'Button "New invoice"' } : picked ? { rect: rect("#outstanding-total"), label: 'Group "Outstanding"' } : { rect: null, label: "" },
    marks: picked ? [{ number: 1, rect: button, kind: "element" }, { number: 2, rect: overdue, kind: "element" }] : [],
    area: state === "area" || state === "recording" ? { rect: area, recording: state === "recording" } : { rect: null, recording: false },
  });

  const mount = (id: string, x: number, y: number, w: number, h: number, node: React.ReactNode) => {
    const host = document.getElementById(id) as HTMLElement;
    Object.assign(host.style, { left: `${x}px`, top: `${y}px`, width: `${w}px`, height: `${h}px` });
    createRoot(host).render(node);
  };
  createRoot(document.getElementById("overlay") as HTMLElement).render(<Overlay />);
  mount("bar", (innerWidth - BAR.w) / 2, innerHeight - BAR.h - 40, BAR.w, BAR.h, <Bar />);
  // Where the app's core puts it (`note_spot` in src-tauri/src/app.rs), for this pick on this page.
  const under = { x: Math.min(overdue.x, innerWidth - NOTE.w - 8), y: overdue.y + overdue.height + 8 };
  const slim = noteStyle === "line" || noteStyle === "ghost";
  // The one-line box says how tall it needs to be, as it tells the core in the app.
  if (slim) platform.resizeNote = async (height) => void ((document.getElementById("note") as HTMLElement).style.height = `${height}px`);
  if (query.get("faint")) useStore.setState({ faint: true });
  // With the comment typed in the bar there is no box, and the bar is wider (DOCK_WIDTH), from its grip.
  if (noteStyle === "dock") (document.getElementById("bar") as HTMLElement).style.width = "644px";
  else mount("note", under.x, under.y, NOTE.w, slim ? 60 : NOTE.h, <Note />);
  if (query.get("tab")) {
    const host = document.body.appendChild(document.createElement("div"));
    host.id = "tab";
    host.style.position = "absolute";
    mount("tab", (innerWidth - BAR.w) / 2 - 72, innerHeight - BAR.h - 40 + 4, 56, 56, <TabMark />);
  }
  if (state === "history") {
    // A window of its own in the app; here a panel in the middle of the page.
    const panel = document.getElementById("history") as HTMLElement;
    panel.hidden = false;
    createRoot(panel).render(<History api={examples} />);
  }
  setTimeout(() => (document.body.dataset.ready = "1"), 150);
};
// The page inside may have finished loading before this script ran.
if (frame.contentDocument?.readyState === "complete" && frame.contentDocument.querySelector("#new-invoice")) start();
else frame.addEventListener("load", start);
