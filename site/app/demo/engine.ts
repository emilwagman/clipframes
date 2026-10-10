// The demos on the page: each is the app's real interface (desktop/ui) over its own Northwind.
//
// The app keeps one store per page, so only one demo can be live at a time. That suits the
// page: a visitor uses one at a time, and the page then carries one copy of the interface
// instead of a frame with its own copy per demo. Every demo has its own picker (web.ts) and
// keeps its own round; the one in view, or under the pointer, is the one whose round is in the
// store and drawn by the app's components. A demo that is not live keeps a still copy of what
// it last showed, so it never goes blank.

import { create } from "zustand";
import type { AreaView, HoverView, MarkView, RoundView, Tool } from "@desktop/ui/platform";
import { useStore as useApp } from "@desktop/ui/store";
import { webPlatform } from "@desktop/ui/web";
import type { Stage, WebPlatform } from "@desktop/ui/web";
import type { Entry } from "@desktop/src/History";
import { track } from "@/lib/analytics";
import { direct } from "./touch";

export type DemoId = "hero" | "picks" | "area" | "clip" | "tab";

/// What the copied text says the page is. The app reads this from the browser's window.
const WHERE = 'Google Chrome "Invoices"';
const PLACE = "Google Chrome · localhost:3000";

/// A round the visitor made, as History lists it.
export interface Capture extends Entry {
  text: string;
}

/// How a scene that is not a web page words what is picked: the made-up desktop app is named
/// the way the app names a native window's parts on Windows. The interface is the same; only
/// the words it is handed differ.
export interface Dialect {
  /// `Label "Play a sound"` as the app says it there: `CheckBox "Play a sound"`.
  label(label: string): string;
  /// The copied text for a round.
  text(round: RoundView): string;
}

export interface Demo {
  id: DemoId;
  dialect: Dialect | null;
  /// What the picker is told about the page. A scene that changes what it shows says so here: `where` is read at every pick.
  stage: Stage;
  /// The element of the page the picker last asked about: under the pointer, or just picked.
  pointed: () => Element | null;
  /// The box the demo is in, for telling how much of it is on screen.
  host: HTMLElement;
  page: HTMLElement;
  glass: HTMLElement;
  platform: WebPlatform;
  tool: Tool;
  opens: boolean;
  round: RoundView;
  hover: HoverView;
  marks: MarkView[];
  area: AreaView;
  /// A still copy of the interface, shown while another demo is the live one.
  still: HTMLElement | null;
  /// The visitor is using this demo; the example does not start by itself any more.
  taken: boolean;
  /// The example has played to its end.
  played: boolean;
  /// What is picked is the visitor's own: they picked it, or went on with what the example picked.
  own: boolean;
  /// The example's picks are being taken away, which is not something the visitor copied.
  clearing: boolean;
  /// Stops the example that is playing.
  stop: (() => void) | null;
  /// How many rounds have been started, to tell the visitor's captures apart.
  rounds: number;
  /// Counted once each per visit: the visitor took the demo over, and did what it is for.
  started: boolean;
  finished: boolean;
}

interface Site {
  live: DemoId | null;
  rounds: Partial<Record<DemoId, RoundView>>;
  playing: Partial<Record<DemoId, boolean>>;
  taken: Partial<Record<DemoId, boolean>>;
  /// The text that is on the visitor's own clipboard, once a demo has put one there.
  clipboard: string | null;
  captures: Capture[];
  /// How many times a demo has been made the live one. A page that is drawn again without being
  /// loaded again (the preview's own pages) sets its demos up anew under the same names, and the
  /// interface has to be drawn for the new one.
  registered: number;
}

export const useSite = create<Site>(() => ({ live: null, rounds: {}, playing: {}, taken: {}, clipboard: null, captures: [], registered: 0 }));

const demos = new Map<DemoId, Demo>();
const byGlass = new WeakMap<HTMLElement, Demo>();
let live: Demo | null = null;
let hovered: Demo | null = null;
const players = new Map<DemoId, (demo: Demo, signal: AbortSignal) => Promise<void>>();

export const demo = (id: DemoId): Demo | undefined => demos.get(id);
export const liveDemo = (): Demo | null => live;

/// "2026-10-09 11:42", the way History writes when a capture was made.
function stamp(now: Date): string {
  const two = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())} ${two(now.getHours())}:${two(now.getMinutes())}`;
}

/// Puts text on the visitor's clipboard, the way the app does after every pick.
export function copyToClipboard(text: string): void {
  // A page shown inside another page may be refused the clipboard; the text on the page is still right.
  try {
    navigator.clipboard.writeText(text).then(() => useSite.setState({ clipboard: text }), () => {});
  } catch {
    /* no clipboard here */
  }
}

/// A round of the visitor's is a capture: History lists it, and the visitor's clipboard gets it.
/// The example's rounds are neither: a page that plays by itself has no business with the clipboard.
function keep(d: Demo, text: string): void {
  const { captures, clipboard } = useSite.getState();
  const id = `${d.id}-${d.rounds}`;
  const first = d.round.picks[0];
  const title = d.round.picks.length > 1 ? `${first.headline} and ${d.round.picks.length - 1} more` : first.headline;
  const before = captures.find((c) => c.id === id);
  const capture: Capture = { id, title, when: before?.when ?? stamp(new Date()), count: d.round.picks.length, images: [], text };
  useSite.setState({ captures: [capture, ...captures.filter((c) => c !== before)] });
  if (text !== clipboard) copyToClipboard(text);
}

export function register(id: DemoId, parts: { host: HTMLElement; page: HTMLElement; glass: HTMLElement }, options: { tool?: Tool; opens?: boolean; history?: boolean } = {}): Demo {
  // The same glass twice is the same demo: React mounts twice while developing.
  const known = byGlass.get(parts.glass);
  if (known) return known;

  const { glass, page } = parts;
  let pointed: Element | null = null;
  const stage: Stage = {
    glass,
    elementAt: (x, y) => {
      const box = glass.getBoundingClientRect();
      pointed = document.elementsFromPoint(box.left + x, box.top + y).find((element) => element !== page && page.contains(element)) ?? null;
      return pointed;
    },
    rectOf: (element) => {
      const box = glass.getBoundingClientRect();
      const r = element.getBoundingClientRect();
      return { x: r.x - box.x, y: r.y - box.y, width: r.width, height: r.height };
    },
    // The demo's own Northwind is the page: the other demos' buttons are not counted with its own.
    root: page,
    place: PLACE,
    where: WHERE,
    copy: () => {},
  };
  const platform = webPlatform(stage);
  // The clock in the bar goes to the History further down the page, on a page that has one.
  platform.history = options.history ?? true;
  platform.openHistory = async () => document.getElementById("history")?.scrollIntoView({ behavior: matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth", block: "center" });

  const d: Demo = {
    id, dialect: null, stage, pointed: () => pointed, ...parts, platform, tool: options.tool ?? "element", opens: options.opens ?? true,
    round: { ...EMPTY, place: { name: PLACE, auto: true } }, hover: { rect: null, label: "" }, marks: [], area: { rect: null, recording: false },
    still: null, taken: false, played: false, own: false, clearing: false, stop: null, rounds: 0, started: false, finished: false,
  };
  byGlass.set(glass, d);
  demos.set(id, d);

  // What the picker says goes to the app's store only while this demo is the live one.
  platform.onRound((heard) => {
    const round = d.dialect ? { ...heard, picks: heard.picks.map((pick) => ({ ...pick, headline: d.dialect?.label(pick.headline) ?? pick.headline })), reference: d.dialect.text(heard) } : heard;
    const changed = round.reference !== d.round.reference;
    d.round = round;
    if (live === d) useApp.setState({ round });
    useSite.setState((s) => ({ rounds: { ...s.rounds, [id]: round } }));
    if (changed && round.reference && d.taken && !d.clearing) keep(d, round.reference);
    // The visitor has done what the demo is for: picked something, or opened the bar from the tab.
    if (d.taken && !d.clearing && !d.finished && (id === "tab" ? round.picking : changed && round.reference !== "")) {
      d.finished = true;
      track("demo_finished", { demo: id });
    }
  });
  platform.onHover((heard) => {
    const hover = d.dialect && heard.label ? { ...heard, label: d.dialect.label(heard.label) } : heard;
    d.hover = hover;
    if (live === d) useApp.setState({ hover });
  });
  platform.onMarks((marks) => {
    d.marks = marks;
    if (live === d) useApp.setState({ marks });
  });
  platform.onArea((area) => {
    d.area = area;
    if (live === d) useApp.setState({ area });
  });

  // A script's pointer is not one the browser knows, so it cannot be captured; the picker asks anyway.
  const capture = glass.setPointerCapture.bind(glass);
  glass.setPointerCapture = (pointer) => {
    try {
      capture(pointer);
    } catch {
      /* not a real pointer */
    }
  };

  direct(d, () => take(d));
  const over = (event: PointerEvent) => {
    if (!event.isTrusted || event.pointerType === "touch") return;
    hovered = d;
    choose();
  };
  d.host.addEventListener("pointerenter", over);
  d.host.addEventListener("pointerleave", () => hovered === d && (hovered = null));
  // Moving the pointer on purpose, pressing, or typing takes the demo over from the example.
  let from: { x: number; y: number } | null = null;
  d.host.addEventListener("pointermove", (event) => {
    if (!event.isTrusted || event.pointerType === "touch" || d.taken) return;
    // The browser reports a move when the page scrolls under a pointer that is lying still.
    from ??= { x: event.screenX, y: event.screenY };
    if (Math.hypot(event.screenX - from.x, event.screenY - from.y) > 12) take(d);
  }, true);
  d.host.addEventListener("pointerleave", () => (from = null));
  d.host.addEventListener("pointerdown", (event) => {
    if (!event.isTrusted || event.pointerType === "touch") return;
    take(d);
    // A press on the page is a pick of the visitor's own. A press in the comment box the example
    // left open is the visitor going on with the example's pick. The bar's buttons are neither.
    const on = event.target instanceof Element ? event.target.closest(".window") : null;
    if (on === null && event.target instanceof Element && glass.contains(event.target)) fresh(d);
    else if (on?.querySelector(".note")) d.own = true;
  }, true);
  d.host.addEventListener("keydown", (event) => {
    if (!event.isTrusted) return;
    take(d);
    d.own = true;
  }, true);
  // The comment box takes the keyboard when it opens, and the browser scrolls to it. That is
  // right when the visitor picked something. When the example did, the keyboard stays where
  // it was and the page stays still.
  d.host.addEventListener("focusin", (event) => {
    if (d.taken || !(event.target instanceof HTMLTextAreaElement)) return;
    const { scrollX, scrollY } = window;
    event.target.blur();
    queueMicrotask(() => window.scrollY !== scrollY && window.scrollTo(scrollX, scrollY));
  });

  void reset(d);
  choose();
  return d;
}

const EMPTY: RoundView = { picking: false, tool: "element", picks: [], noting: null, recording: null, reference: "", trouble: null, shortcut: "Esc", place: null };

/// Back to how the demo starts: the bar up with its tool on, or only the tab.
export async function reset(d: Demo): Promise<void> {
  d.stop?.();
  d.stop = null;
  if (d.round.picking) await d.platform.done();
  if (d.opens) open(d);
}

export function open(d: Demo): void {
  d.rounds += 1;
  d.platform.open();
  if (d.tool !== "element") void d.platform.setTool(d.tool);
}

/// The visitor is using the demo: the example stops where it is.
export function take(d: Demo): void {
  if (d.taken) return;
  d.taken = true;
  d.stop?.();
  d.stop = null;
  if (!d.started) track("demo_started", { demo: d.id });
  d.started = true;
  useSite.setState((s) => ({ taken: { ...s.taken, [d.id]: true } }));
  if (live !== d) activate(d);
}

/// The visitor's first pick starts a round of their own: what the example picked is taken away
/// first, so the text is exactly what they did. Called before the pick reaches the picker.
export function fresh(d: Demo): void {
  if (d.own) return;
  d.own = true;
  d.clearing = true;
  for (let i = d.round.picks.length - 1; i >= 0; i--) void d.platform.removePick(i);
  d.clearing = false;
}

/// Hands the round back to the example and plays it from the start.
export async function replay(d: Demo): Promise<void> {
  d.taken = false;
  d.own = false;
  d.played = false;
  useSite.setState((s) => ({ taken: { ...s.taken, [d.id]: false } }));
  await reset(d);
  if (live !== d) activate(d);
  else play(d);
}

export function setPlayer(id: DemoId, player: (demo: Demo, signal: AbortSignal) => Promise<void>): void {
  players.set(id, player);
}

function play(d: Demo): void {
  const player = players.get(d.id);
  if (!player || d.taken || d.played || d.stop) return;
  const control = new AbortController();
  const stop = () => control.abort();
  d.stop = stop;
  useSite.setState((s) => ({ playing: { ...s.playing, [d.id]: true } }));
  player(d, control.signal)
    .then(() => (d.played = true), () => {})
    .finally(() => {
      // Unless it was stopped and has already been started again.
      if (d.stop !== stop && d.stop !== null) return;
      d.stop = null;
      useSite.setState((s) => ({ playing: { ...s.playing, [d.id]: false } }));
    });
}

function freeze(d: Demo): void {
  const ui = d.glass.querySelector(":scope > .clipframes-web");
  if (!ui) return;
  const still = ui.cloneNode(true) as HTMLElement;
  // What was typed is not part of the markup, so it is carried over by hand.
  const typed = ui.querySelector<HTMLTextAreaElement>("#comment");
  const copy = still.querySelector<HTMLTextAreaElement>("#comment");
  if (typed && copy) {
    copy.value = typed.value;
    copy.removeAttribute("id");
  }
  still.inert = true;
  still.dataset.still = "";
  d.glass.append(still);
  d.still = still;
}

export function activate(next: Demo): void {
  if (live === next) return;
  const before = live;
  if (before) {
    // An example that is cut short starts over the next time; one the visitor took over stays.
    if (before.stop && !before.taken) {
      void reset(before);
    }
    freeze(before);
  }
  live = next;
  useApp.setState({ platform: next.platform, round: next.round, hover: next.hover, marks: next.marks, area: next.area });
  useSite.setState((s) => ({ live: next.id, registered: s.registered + 1 }));
}

/// Called once the app's components are drawn for the live demo: its still copy can go, and
/// the picker says again where things are, for the comment box.
export function onLive(id: DemoId): void {
  const d = demos.get(id);
  if (!d || live !== d) return;
  d.still?.remove();
  d.still = null;
  void d.platform.setAuto(d.round.place?.auto ?? true);
  play(d);
}

/// How much of a demo is on screen, from 0 to 1.
function shown(d: Demo): number {
  const r = d.host.getBoundingClientRect();
  const visible = Math.min(r.bottom, innerHeight) - Math.max(r.top, 0);
  return Math.max(0, visible) / Math.min(r.height, innerHeight);
}

let queued = false;
/// Picks the live demo: the one under the pointer, otherwise the one nearest the middle of the screen.
export function choose(): void {
  if (queued) return;
  queued = true;
  requestAnimationFrame(() => {
    queued = false;
    if (hovered && shown(hovered) > 0.2) return activate(hovered);
    let best: Demo | null = null;
    let nearest = Infinity;
    for (const d of demos.values()) {
      if (shown(d) < 0.5) continue;
      const r = d.host.getBoundingClientRect();
      const distance = Math.abs(r.top + r.height / 2 - innerHeight / 2);
      if (distance < nearest) [best, nearest] = [d, distance];
    }
    // At the top of the page the first demo is only partly on screen, and is still the one to start with.
    if (!best && !live) best = [...demos.values()].filter((d) => shown(d) > 0.15).sort((a, b) => shown(b) - shown(a))[0] ?? null;
    if (best) activate(best);
  });
}

if (typeof window !== "undefined") {
  window.addEventListener("scroll", choose, { passive: true });
  window.addEventListener("resize", choose);
}
