// The real bar, comment box and overlay over the demo page, in one browser window, so a look
// can be judged without running the app. ?state=empty shows the bar before anything is picked.
import "../ui/style.css";
import { mountBar } from "../ui/bar";
import { mountNote } from "../ui/note";
import { mountOverlay } from "../ui/overlay";
import type { HoverView, MarkView, Platform, Rect, RoundView } from "../ui/platform";

const params = new URLSearchParams(location.search);
const empty = params.get("state") === "empty";
const BAR = { w: Number(params.get("barw") ?? 640), h: 80 };
const NOTE = { w: 380, h: 158 };

const frame = document.getElementById("page") as HTMLIFrameElement;
frame.addEventListener("load", () => {
  const doc = frame.contentDocument!;
  const rect = (selector: string): Rect => {
    const r = doc.querySelector(selector)!.getBoundingClientRect();
    return { x: r.x, y: r.y, width: r.width, height: r.height };
  };
  const button = rect("#new-invoice");
  const overdue = rect("#overdue-total strong");
  const hovered = rect("#outstanding-total");

  const round: RoundView = {
    picking: true,
    picks: empty ? [] : [
      { headline: 'Button "New invoice"', selector: "#new-invoice .btn.btn-primary", note: "make this green" },
      { headline: 'Text "$3,120"', selector: "#overdue-total .stat.overdue", note: "too alarming, use the normal text colour" },
    ],
    noting: empty ? null : 1,
    reference: "",
    trouble: null,
    shortcut: "Ctrl+Shift+Space",
  };
  let onRound: ((r: RoundView) => void)[] = [];
  let onHover: (h: HoverView) => void = () => {};
  let onMarks: (m: MarkView[]) => void = () => {};
  const platform: Platform = {
    tools: ["element", "area", "clip"],
    history: true,
    state: async () => round,
    onRound: (l) => onRound.push(l),
    onHover: (l) => (onHover = l),
    onMarks: (l) => (onMarks = l),
    setNote: async () => {}, closeNote: async () => {}, removePick: async () => {}, done: async () => {}, escape: async () => {}, openPermission: async () => {},
  };

  const place = (id: string, x: number, y: number, w: number, h: number) => {
    const node = document.getElementById(id)!;
    Object.assign(node.style, { left: `${x}px`, top: `${y}px`, width: `${w}px`, height: `${h}px` });
    return node;
  };
  mountOverlay(document.getElementById("overlay")!, platform);
  mountBar(place("bar", (innerWidth - BAR.w) / 2, innerHeight - BAR.h - 40, BAR.w, BAR.h), platform);
  const note = place("note", overdue.x, overdue.y + overdue.height + 8, NOTE.w, NOTE.h);
  mountNote(note, platform);
  note.hidden = empty;

  onRound.forEach((l) => l(round));
  onHover(empty ? { rect: button, label: 'Button "New invoice"' } : { rect: hovered, label: 'Group "Outstanding"' });
  onMarks(empty ? [] : [{ number: 1, rect: button }, { number: 2, rect: overdue }]);
  document.body.dataset.ready = "1";
});
