// A copy of ui/web.ts with what the fix prototypes need added, each behind an option so one
// fix can be judged at a time. ui/web.ts itself is unchanged.
//
// Added: choosing the parent or child of what is hovered, reopening an earlier pick from its
// number, using the page without ending the round, marks that can be frozen where they were
// picked (what the desktop app does today), scrolling the page under the glass, and taking a
// closed round up again.

import { EMPTY_ROUND } from "../../ui/platform";
import type { AreaView, HoverView, MarkView, PickView, Rect, RoundView, Tool } from "../../ui/platform";
import { reference, selector } from "../../ui/web";
import { brief, named as headline } from "./names";
import type { Stage, WebPlatform } from "../../ui/web";

export interface Options {
  tools?: Tool[];
  shortcut?: string;
  /** Arrow up and down step out to the parent and back in. */
  levels?: boolean;
  /** A click on a pick's number opens its comment again. */
  badges?: boolean;
  /** "fixed": marks stay where they were on screen, as in the desktop app today. */
  marks?: "follow" | "fixed";
  /** Windows whose scrolling moves the marks. */
  windows?: Window[];
}

/** The elements around the hovered one, outermost first, and which of them is chosen. */
export interface Trail {
  names: string[];
  chosen: number;
  rect: Rect;
}

export interface Platform2 extends WebPlatform {
  openNote(index: number): void;
  /** Takes the round that was just closed up again, with its picks. */
  reopen(): void;
  /** While on, the pointer belongs to the page: clicks reach it and nothing is picked. */
  setThrough(on: boolean): void;
  onThrough(listener: (on: boolean) => void): void;
  onTrail(listener: (trail: Trail | null) => void): void;
  /** Steps out to the parent (1) or back in (-1). */
  step(by: number): void;
}

const span = (a: { x: number; y: number }, b: { x: number; y: number }): Rect => ({ x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), width: Math.abs(a.x - b.x), height: Math.abs(a.y - b.y) });
const inside = (r: Rect, p: { x: number; y: number }) => p.x >= r.x && p.x < r.x + r.width && p.y >= r.y && p.y < r.y + r.height;
/** Where a mark's number sits (ui/style.css, .mark b). */
const badge = (r: Rect): Rect => ({ x: r.x - 13, y: r.y - 13, width: 28, height: 28 });

export function webPlatform2(stage: Stage, options: Options = {}): Platform2 {
  let round: RoundView = { ...EMPTY_ROUND, shortcut: options.shortcut ?? "Esc", place: stage.place ? { name: stage.place, auto: true } : null };
  let targets: (Element | Rect)[] = [];
  // Where each pick was on screen when it was made.
  let frozen: Rect[] = [];
  let last: { picks: PickView[]; targets: (Element | Rect)[]; frozen: Rect[] } | null = null;
  let area: AreaView = { rect: null, recording: false };
  let drag: { x: number; y: number } | null = null;
  let timer: number | undefined;
  let through = false;
  // The element under the pointer, and how many parents out from it the user has stepped.
  let base: Element | null = null;
  let level = 0;
  let point: { x: number; y: number } | null = null;
  const on = { round: [] as ((r: RoundView) => void)[], hover: [] as ((h: HoverView) => void)[], marks: [] as ((m: MarkView[]) => void)[], area: [] as ((a: AreaView) => void)[], note: [] as ((r: Rect | null) => void)[], close: [] as ((s: string) => void)[], through: [] as ((t: boolean) => void)[], trail: [] as ((t: Trail | null) => void)[] };

  const gone = (target: Element | Rect) => "tagName" in target && !target.isConnected;
  const rectOf = (index: number): Rect => {
    const target = targets[index];
    return options.marks === "fixed" || !("tagName" in target) ? frozen[index] : stage.rectOf(target);
  };
  const publish = (change: Partial<RoundView> = {}) => {
    round = { ...round, ...change };
    round.reference = reference(round.picks, stage.where);
    on.round.forEach((f) => f(round));
    // A pick whose element has left the page keeps its number and has no mark.
    const marks = targets.map((t, i) => ({ number: i + 1, rect: rectOf(i), kind: round.picks[i].kind, t })).filter((m) => !gone(m.t) && m.rect.width > 0);
    on.marks.forEach((f) => f(marks.map(({ number, rect, kind }) => ({ number, rect, kind }))));
    on.note.forEach((f) => f(round.noting === null || gone(targets[round.noting]) ? null : rectOf(round.noting)));
  };
  const setHover = (next: HoverView) => on.hover.forEach((f) => f(next));
  const setTrail = (next: Trail | null) => on.trail.forEach((f) => f(next));
  const setArea = (next: AreaView) => on.area.forEach((f) => f((area = next)));
  const add = (pick: PickView, target: Element | Rect) => {
    targets.push(target);
    frozen.push("tagName" in target ? stage.rectOf(target) : target);
    publish({ picks: [...round.picks, pick], noting: round.picks.length });
  };
  const at = (event: MouseEvent) => {
    const box = stage.glass.getBoundingClientRect();
    return { x: event.clientX - box.left, y: event.clientY - box.top };
  };

  // What a click at this point picks: the element there, or one of its parents.
  const chain = (element: Element): Element[] => {
    const out: Element[] = [];
    for (let e: Element | null = element; e && e.tagName !== "BODY" && e.tagName !== "HTML"; e = e.parentElement) out.push(e);
    return out;
  };
  const chosen = (p: { x: number; y: number }): Element | null => {
    const element = stage.elementAt(p.x, p.y);
    if (!element || !options.levels) return element;
    if (element !== base) {
      base = element;
      level = 0;
    }
    const up = chain(element);
    level = Math.max(0, Math.min(level, up.length - 1));
    return up[level] ?? element;
  };
  const badgeAt = (p: { x: number; y: number }) => (options.badges ? targets.findIndex((t, i) => !gone(t) && inside(badge(rectOf(i)), p)) : -1);
  const hoverAt = (p: { x: number; y: number }) => {
    const earlier = badgeAt(p);
    if (earlier >= 0) {
      setTrail(null);
      return setHover({ rect: rectOf(earlier), label: `Edit ${earlier + 1}` });
    }
    const element = chosen(p);
    setHover(element ? { rect: stage.rectOf(element), label: headline(element) } : { rect: null, label: "" });
    if (options.levels) {
      const up = base ? chain(base) : [];
      // Innermost first: the element under the pointer and a few of its parents, always one
      // more than the chosen one so it shows there is further to go. Drawn outermost first.
      const shown = up.slice(0, Math.max(level + 2, 4));
      const skip = Math.max(0, shown.length - 5);
      const names = shown.slice(skip).map(brief).reverse();
      setTrail(element && up.length > 1 ? { names, chosen: names.length - 1 - (level - skip), rect: stage.rectOf(element) } : null);
    }
  };
  const clearHover = () => {
    setHover({ rect: null, label: "" });
    setTrail(null);
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
  const own = (event: Event) => event.target instanceof Element && event.target.closest(".window") !== null;
  glass.addEventListener("pointermove", (event) => {
    if (!round.picking) return;
    if (own(event) && !drag) return clearHover();
    point = at(event);
    if (drag) return setArea({ rect: span(drag, point), recording: false });
    if (round.tool !== "element") return;
    hoverAt(point);
  });
  glass.addEventListener("pointerleave", () => !drag && clearHover());
  glass.addEventListener("pointerdown", (event) => {
    if (!round.picking || event.button !== 0 || own(event)) return;
    event.preventDefault();
    const p = at(event);
    if (round.tool === "element") {
      const earlier = badgeAt(p);
      if (earlier >= 0) {
        clearHover();
        return publish({ noting: earlier });
      }
      const element = chosen(p);
      if (!element) return;
      clearHover();
      level = 0;
      return add({ kind: "element", headline: headline(element), selector: selector(element), note: "" }, element);
    }
    drag = p;
    glass.setPointerCapture(event.pointerId);
    if (round.noting !== null) publish({ noting: null });
  });
  glass.addEventListener("pointerup", (event) => {
    if (!drag) return;
    const rect = span(drag, at(event));
    drag = null;
    if (rect.width < 8 || rect.height < 8) return setArea({ rect: null, recording: false });
    if (round.tool === "area") {
      setArea({ rect: null, recording: false });
      return add({ kind: "area", headline: "Screenshot", selector: "", note: "" }, rect);
    }
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
  // In the desktop app the wheel goes to the app underneath. Here the glass is in the way, so
  // the page is scrolled by hand.
  glass.addEventListener(
    "wheel",
    (event) => {
      if (!round.picking || own(event)) return;
      const p = at(event);
      const scroller = stage.elementAt(p.x, p.y)?.ownerDocument.scrollingElement;
      scroller?.scrollBy(event.deltaX, event.deltaY);
      follow();
      if (round.tool === "element" && options.marks !== "fixed") hoverAt(p);
    },
    { passive: true },
  );
  const follow = () => round.picking && publish();
  window.addEventListener("resize", follow);
  window.addEventListener("scroll", follow, true);
  options.windows?.forEach((w) => w.addEventListener("scroll", follow, true));

  const done = async () => {
    await stopRecording();
    const copied = round.reference;
    last = round.picks.length ? { picks: round.picks, targets, frozen } : null;
    targets = [];
    frozen = [];
    through = false;
    on.through.forEach((f) => f(false));
    clearHover();
    setArea({ rect: null, recording: false });
    publish({ picking: false, picks: [], noting: null, tool: "element" });
    glass.style.pointerEvents = "none";
    on.close.forEach((f) => f(copied));
  };

  glass.style.pointerEvents = "none";
  return {
    tools: options.tools ?? ["element", "area", "clip"],
    history: false,
    state: () => Promise.resolve().then(() => round),
    onRound: (f) => void on.round.push(f),
    onHover: (f) => void on.hover.push(f),
    onMarks: (f) => void on.marks.push(f),
    onArea: (f) => void on.area.push(f),
    onNoteAt: (f) => void on.note.push(f),
    onClose: (f) => void on.close.push(f),
    onThrough: (f) => void on.through.push(f),
    onTrail: (f) => void on.trail.push(f),
    open: () => {
      glass.style.pointerEvents = "";
      publish({ picking: true });
    },
    reopen: () => {
      if (!last) return;
      ({ targets, frozen } = last);
      glass.style.pointerEvents = "";
      publish({ picking: true, picks: last.picks, noting: null });
      last = null;
    },
    openNote: (index) => publish({ noting: index }),
    setThrough: (value) => {
      if (!round.picking || round.recording !== null || through === value) return;
      through = value;
      glass.style.pointerEvents = value ? "none" : "";
      clearHover();
      on.through.forEach((f) => f(value));
      if (value && round.noting !== null) publish({ noting: null });
      else publish();
    },
    step: (by) => {
      if (!options.levels || !round.picking || !point || round.noting !== null) return;
      level = Math.max(0, level + by);
      hoverAt(point);
    },
    setTool: async (tool) => {
      clearHover();
      publish({ tool, noting: null });
    },
    stopRecording,
    setNote: async (index, note) => publish({ picks: round.picks.map((p, i) => (i === index ? { ...p, note: note.trim() } : p)) }),
    closeNote: async () => publish({ noting: null }),
    removePick: async (index) => {
      targets.splice(index, 1);
      frozen.splice(index, 1);
      publish({ picks: round.picks.filter((_, i) => i !== index), noting: null });
    },
    setAuto: async (auto) => publish({ place: round.place && { ...round.place, auto } }),
    openHistory: async () => {},
    done,
    escape: async () => (round.recording !== null ? stopRecording() : round.noting !== null ? publish({ noting: null }) : done()),
    openPermission: async () => {},
  };
}
