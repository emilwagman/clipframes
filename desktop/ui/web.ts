// Clipframes over a web page, with no app behind it: the same bar, comment box and overlay,
// picking the page's own elements. The website's demos and the lab's prototypes run on this.
//
// A stage is the page being pointed at and a sheet of glass over it. The glass takes the
// pointer, so a click picks instead of pressing the button underneath.

import { EMPTY_ROUND } from "./platform";
import type { AreaView, HoverView, MarkView, PickView, Platform, Rect, RoundView, Tool } from "./platform";
import { whereabouts } from "./whereabouts";

export interface Stage {
  /** Covers the page and receives the pointer. Rects are in its own pixels. */
  glass: HTMLElement;
  /** The page's element under a point of the glass. */
  elementAt(x: number, y: number): Element | null;
  /** Where an element of the page is, in the glass's pixels. */
  rectOf(element: Element): Rect;
  /** "Google Chrome · localhost:3000": what the pin in the bar is about. */
  place?: string;
  /** What the page is called in the copied text, e.g. `Google Chrome "Invoices"`. */
  where?: string;
  /** The page's outermost element, when the page is a part of the document. Left out, it is the document's body. */
  root?: Element;
  /** Puts the copied text on the clipboard. Left out, the browser's own clipboard is written. */
  copy?(text: string): void;
}

export interface WebPlatform extends Platform {
  /** Starts a round: the bar is up and the page can be pointed at. */
  open(): void;
  /** Where the comment box belongs: under this rect, or nowhere. */
  onNoteAt(listener: (rect: Rect | null) => void): void;
  /** Called when the round ends, with what was copied. */
  onClose(listener: (reference: string) => void): void;
}

const ROLES: Record<string, string> = { button: "Button", a: "Link", input: "Text field", textarea: "Text field", select: "Menu", img: "Image", svg: "Image", table: "Table", tr: "Row", td: "Cell", th: "Cell", nav: "Navigation", li: "Item", label: "Label" };
const TEXT = new Set(["p", "span", "strong", "b", "em", "i", "small", "code", "h1", "h2", "h3", "h4", "h5", "h6"]);

/** `Button "New invoice"`, the way the app names what it reads from the system. */
export function headline(element: Element): string {
  const tag = element.tagName.toLowerCase();
  const role = /^h[1-6]$/.test(tag) ? "Heading" : (ROLES[tag] ?? (TEXT.has(tag) ? "Text" : "Group"));
  const own = element.getAttribute("aria-label") || element.getAttribute("alt") || element.getAttribute("placeholder") || "";
  // A group is named by its first words only when it is small; a whole section has no name.
  const text = (element.textContent ?? "").replace(/\s+/g, " ").trim();
  const name = own || (role === "Group" && text.length > 40 ? "" : text.length > 40 ? `${text.slice(0, 39)}…` : text);
  return name ? `${role} "${name}"` : role;
}

/** "#new-invoice .btn.btn-primary": the nearest id, then the element's own classes. */
export function selector(element: Element): string {
  const id = element.closest("[id]")?.id;
  const classes = [...element.classList].join(".");
  return [id ? `#${id}` : "", classes ? `.${classes}` : ""].filter(Boolean).join(" ");
}

function describe(pick: PickView, number: number): string {
  if (pick.kind === "area") return `Screenshot (${number}.png)`;
  if (pick.kind === "clip") return `${pick.headline} (${number}/)`;
  // What it is, then which one, then under what: the order the app's core says them in.
  const what = pick.selector ? `${pick.headline} (${pick.selector})` : pick.headline;
  return [what, ...(pick.whereabouts ?? [])].join(", ");
}

/** Which of the page's elements that read the same this one is, and the heading it is under. */
function placeOf(element: Element, root?: Element): string[] {
  const said = headline(element);
  // Only something with a name is counted: `Group` alone is said of every plain container.
  const same = said.includes('"') ? (other: Element) => headline(other) === said : null;
  return whereabouts(root ?? (element.ownerDocument.body as Element), element, same);
}

/** What goes on the clipboard: one line for one pick, a numbered list for several. */
export function reference(picks: PickView[], where = ""): string {
  const place = where ? ` in ${where}` : "";
  const note = (p: PickView) => (p.note ? `: ${p.note}` : "");
  if (picks.length === 0) return "";
  if (picks.length === 1) return `[${describe(picks[0], 1)}${place}${note(picks[0])}]`;
  return [`[Clipframes: ${picks.length} things${place}]`, ...picks.map((p, i) => `${i + 1}. ${describe(p, i + 1)}${note(p)}`)].join("\n");
}

const span = (a: { x: number; y: number }, b: { x: number; y: number }): Rect => ({ x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), width: Math.abs(a.x - b.x), height: Math.abs(a.y - b.y) });

export function webPlatform(stage: Stage, options: { tools?: Tool[]; shortcut?: string } = {}): WebPlatform {
  let round: RoundView = { ...EMPTY_ROUND, shortcut: options.shortcut ?? "Esc", place: stage.place ? { name: stage.place, auto: true } : null };
  // Where each pick is on the page: an element, which may move, or a fixed area.
  const targets: (Element | Rect)[] = [];
  let area: AreaView = { rect: null, recording: false };
  let drag: { x: number; y: number } | null = null;
  let timer: number | undefined;
  const copy = stage.copy ?? ((text: string) => void navigator.clipboard?.writeText(text).catch(() => {}));
  const on = { round: [] as ((r: RoundView) => void)[], hover: [] as ((h: HoverView) => void)[], marks: [] as ((m: MarkView[]) => void)[], area: [] as ((a: AreaView) => void)[], note: [] as ((r: Rect | null) => void)[], close: [] as ((s: string) => void)[] };

  // Not instanceof: an element of a page in a frame belongs to that frame's own Element.
  const rectOf = (target: Element | Rect): Rect => ("tagName" in target ? stage.rectOf(target) : target);
  const publish = (change: Partial<RoundView> = {}) => {
    round = { ...round, ...change };
    round.reference = reference(round.picks, stage.where);
    if (round.reference) copy(round.reference);
    on.round.forEach((f) => f(round));
    on.marks.forEach((f) => f(targets.map((t, i) => ({ number: i + 1, rect: rectOf(t), kind: round.picks[i].kind }))));
    on.note.forEach((f) => f(round.noting === null ? null : rectOf(targets[round.noting])));
  };
  const setHover = (next: HoverView) => on.hover.forEach((f) => f(next));
  const setArea = (next: AreaView) => on.area.forEach((f) => f((area = next)));
  const add = (pick: PickView, target: Element | Rect) => {
    targets.push(target);
    publish({ picks: [...round.picks, pick], noting: round.picks.length });
  };
  const at = (event: PointerEvent) => {
    const box = stage.glass.getBoundingClientRect();
    return { x: event.clientX - box.left, y: event.clientY - box.top };
  };

  const stopRecording = async () => {
    if (round.recording === null || !area.rect) return;
    window.clearInterval(timer);
    const seconds = Math.max(1, Math.round(round.recording));
    const rect = area.rect;
    stage.glass.style.pointerEvents = "";
    setArea({ rect: null, recording: false });
    round = { ...round, recording: null };
    add({ kind: "clip", headline: `Screen clip, ${seconds} s, ${seconds * 4} frames`, selector: "", note: "" }, rect);
  };

  const glass = stage.glass;
  // The bar and the comment box sit on the glass; the pointer over them is theirs.
  const own = (event: Event) => event.target instanceof Element && event.target.closest(".window") !== null;
  glass.addEventListener("pointermove", (event) => {
    if (!round.picking) return;
    if (own(event) && !drag) return setHover({ rect: null, label: "" });
    const point = at(event);
    if (drag) return setArea({ rect: span(drag, point), recording: false });
    if (round.tool !== "element") return;
    const element = stage.elementAt(point.x, point.y);
    setHover(element ? { rect: stage.rectOf(element), label: headline(element) } : { rect: null, label: "" });
  });
  glass.addEventListener("pointerleave", () => !drag && setHover({ rect: null, label: "" }));
  glass.addEventListener("pointerdown", (event) => {
    if (!round.picking || event.button !== 0 || own(event)) return;
    event.preventDefault();
    const point = at(event);
    if (round.tool === "element") {
      const element = stage.elementAt(point.x, point.y);
      if (!element) return;
      setHover({ rect: null, label: "" });
      return add({ kind: "element", headline: headline(element), selector: selector(element), note: "", whereabouts: placeOf(element, stage.root) }, element);
    }
    drag = point;
    glass.setPointerCapture(event.pointerId);
    if (round.noting !== null) publish({ noting: null });
  });
  glass.addEventListener("pointerup", (event) => {
    if (!drag) return;
    const rect = span(drag, at(event));
    drag = null;
    // A click without a drag is not an area.
    if (rect.width < 8 || rect.height < 8) return setArea({ rect: null, recording: false });
    if (round.tool === "area") {
      setArea({ rect: null, recording: false });
      return add({ kind: "area", headline: "Screenshot", selector: "", note: "" }, rect);
    }
    // A clip: the page is used as usual while the time runs.
    setArea({ rect, recording: true });
    glass.style.pointerEvents = "none";
    const started = performance.now();
    publish({ recording: 0 });
    timer = window.setInterval(() => {
      const seconds = (performance.now() - started) / 1000;
      if (seconds >= 60) return void stopRecording();
      round = { ...round, recording: seconds };
      on.round.forEach((f) => f(round));
    }, 250);
  });
  // The page scrolls and resizes under the marks.
  const follow = () => round.picking && publish();
  window.addEventListener("resize", follow);
  window.addEventListener("scroll", follow, true);

  const done = async () => {
    await stopRecording();
    const copied = round.reference;
    targets.length = 0;
    setHover({ rect: null, label: "" });
    setArea({ rect: null, recording: false });
    publish({ picking: false, picks: [], noting: null, tool: "element" });
    glass.style.pointerEvents = "none";
    on.close.forEach((f) => f(copied));
  };

  glass.style.pointerEvents = "none";
  return {
    tools: options.tools ?? ["element", "area", "clip"],
    history: false,
    // Read when the answer is delivered, so a round opened in the meantime is not undone.
    state: () => Promise.resolve().then(() => round),
    onRound: (f) => void on.round.push(f),
    onHover: (f) => void on.hover.push(f),
    onMarks: (f) => void on.marks.push(f),
    onArea: (f) => void on.area.push(f),
    onNoteAt: (f) => void on.note.push(f),
    onClose: (f) => void on.close.push(f),
    open: () => {
      glass.style.pointerEvents = "";
      publish({ picking: true });
    },
    setTool: async (tool) => {
      setHover({ rect: null, label: "" });
      publish({ tool, noting: null });
    },
    stopRecording,
    setNote: async (index, note) => publish({ picks: round.picks.map((p, i) => (i === index ? { ...p, note: note.trim() } : p)) }),
    closeNote: async () => publish({ noting: null }),
    removePick: async (index) => {
      targets.splice(index, 1);
      publish({ picks: round.picks.filter((_, i) => i !== index), noting: null });
    },
    setAuto: async (auto) => publish({ place: round.place && { ...round.place, auto } }),
    openHistory: async () => {},
    done,
    escape: async () => (round.recording !== null ? stopRecording() : round.noting !== null ? publish({ noting: null }) : done()),
    openPermission: async () => {},
  };
}
