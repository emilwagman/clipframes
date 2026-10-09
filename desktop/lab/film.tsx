// The launch film: desktop/film.html. Clipframes runs for real over a simplified page (ui/web.ts
// and the app's own components), and a script moves a drawn pointer and sends the pointer and
// keyboard events a person would. The outline, the label, the numbers, the comment box and the
// copied text are whatever the interface does in answer.
//
// The film is a function of the frame number: step(n) must be called for every frame in order,
// since the interface keeps state. tools/film-render.mjs does that and photographs each frame;
// opened in a browser the page steps itself in real time.
import { createRoot } from "react-dom/client";
import "../ui/style.css";
import { connect } from "../ui/store";
import { OnPage } from "../ui/OnPage";
import { webPlatform } from "../ui/web";

const FPS = 60;
const query = new URLSearchParams(location.search);
const browser = query.get("window") === "browser";
if (browser) document.documentElement.dataset.window = "browser";
if (query.get("look")) document.documentElement.dataset.look = query.get("look") as string;

interface Point { x: number; y: number }
/** What the picture shows of the world: a 16:9 view this wide, from this corner. */
interface Camera { x: number; y: number; w: number }

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const picture = $("frame");
const world = $("world");
const glass = $("glass");
const frame = $<HTMLIFrameElement>("page");
const pointer = document.getElementById("pointer") as unknown as SVGElement;
const caret = $("caret");
const pasted = $("pasted");
const cursor = $("cursor");

// What the interface put on the clipboard, kept here: the film must not touch the real clipboard
// of the computer it is rendered on.
let clipboard = "";
Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: async (text: string) => void (clipboard = text) } });
let closedWith = "";

// ---------- the script ----------

// Rest to rest on minimum jerk, for everything that travels.
const ease = (u: number) => (u <= 0 ? 0 : u >= 1 ? 1 : u * u * u * (10 - 15 * u + 6 * u * u));
const mix = (a: number, b: number, u: number) => a + (b - a) * u;

const WIDE: Camera = { x: 0, y: 0, w: 1024 };
const cameras: { t0: number; t1: number; to: Camera }[] = [];
const moves: { t0: number; t1: number; to: () => Point; bend: number; from?: Point; dest?: Point }[] = [];
const events: { frame: number; run: () => void }[] = [];
const at = (t: number, run: () => void) => events.push({ frame: Math.round(t * FPS), run });

/** The middle of something in the page, or a point inside it, in the world's pixels. */
const inPage = (selector: string, fx = 0.5, fy = 0.5) => (): Point => {
  const r = (frame.contentDocument as Document).querySelector(selector)!.getBoundingClientRect();
  return { x: frame.offsetLeft + $("app").offsetLeft + r.x + r.width * fx, y: frame.offsetTop + $("app").offsetTop + r.y + r.height * fy };
};
/** The middle of something of the film's own document, whatever the camera is doing. */
const onStage = (selector: string, fx = 0.5, fy = 0.5) => (): Point => {
  const r = document.querySelector(selector)!.getBoundingClientRect();
  const w = world.getBoundingClientRect();
  const s = w.width / world.offsetWidth;
  return { x: (r.x - w.x + r.width * fx) / s, y: (r.y - w.y + r.height * fy) / s };
};

// Keys land unevenly, the same way every time.
let seed = 7;
const random = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 4294967296);
function type(text: string, start: number, per: number): number {
  let t = start;
  for (const key of text) {
    at(t, () => press(key));
    t += per * (0.7 + 0.6 * random()) * (key === " " ? 1.25 : 1);
  }
  return t;
}

const REST: Point = { x: 566, y: 396 };

// The window, still; then the shortcut, and the bar rises.
const OPEN = 1.0;
cameras.push({ t0: 0, t1: 1.7, to: { x: 12, y: 4, w: 1000 } });
at(OPEN, () => window.dispatchEvent(new KeyboardEvent("keydown", { key: " ", code: "Space", ctrlKey: true, shiftKey: true })));
// To the button, with the camera going in on it from the same frame.
moves.push({ t0: 1.75, t1: 2.9, to: inPage("#new-invoice", 0.8, 0.72), bend: 0.16 });
cameras.push({ t0: 1.75, t1: 2.9, to: { x: 384, y: 10, w: 640 } });
at(3.3, click);
const typed = type("make this green", 3.65, 0.085);
at(typed + 0.28, () => press("Enter"));
// Down to the bar for the area tool, then a drag over the rows.
const NEXT = typed + 0.4;
moves.push({ t0: NEXT, t1: NEXT + 1.05, to: onStage('.bar [aria-label="Screenshot an area"]', 0.55, 0.55), bend: -0.1 });
cameras.push({ t0: NEXT, t1: NEXT + 1.15, to: { x: 228, y: 160, w: 700 } });
at(NEXT + 1.15, click);
moves.push({ t0: NEXT + 1.25, t1: NEXT + 2.1, to: inPage("tbody", 0, 0), bend: 0.12 });
const DRAG = NEXT + 2.2;
at(DRAG, down);
moves.push({ t0: DRAG + 0.05, t1: DRAG + 1.4, to: inPage("tbody", 1, 1), bend: 0.05 });
at(DRAG + 1.48, up);
const typed2 = type("rows are too tall", DRAG + 1.75, 0.064);
at(typed2 + 0.22, () => press("Enter"));
// Close: everything picked is on the clipboard.
const CLOSE = typed2 + 0.4;
moves.push({ t0: CLOSE, t1: CLOSE + 0.8, to: onStage('.bar [aria-label="Close"]', 0.5, 0.55), bend: 0.14 });
at(CLOSE + 0.95, click);
// Over to the terminal, and paste.
const OVER = CLOSE + 1.1;
cameras.push({ t0: OVER, t1: OVER + 1.15, to: { x: 752, y: 0, w: 1024 } });
moves.push({ t0: OVER + 0.05, t1: OVER + 1.0, to: onStage("#term .prompt", 0.42, 0.62), bend: -0.1 });
at(OVER + 1.1, click);
at(OVER + 1.1, () => (cursor.hidden = false));
const PASTE = OVER + 1.4;
at(PASTE, () => (pasted.textContent = clipboard));
cameras.push({ t0: PASTE + 0.15, t1: PASTE + 2.2, to: { x: 1034, y: 74, w: 760 } });
moves.push({ t0: PASTE + 0.2, t1: PASTE + 0.9, to: () => ({ x: 1640, y: 404 }), bend: 0.1 });
const END = Number(query.get("end") ?? PASTE + 3.2);

// ---------- the hands ----------

let where: Point = REST;
let held = false;
let pressedAt = -1;
let now = 0;
let hovered: Element | null = null;
// The fastest the pointer crosses the picture, in pixels a second: a check on the script.
let fastest = 0;
let seen: Point | null = null;
const speeds: number[] = [];

/** Where a point of the world is in the browser window. */
function client(p: Point): Point {
  const w = world.getBoundingClientRect();
  const s = w.width / world.offsetWidth;
  return { x: w.x + p.x * s, y: w.y + p.y * s };
}

// A pointer event at a point of the world. It goes to whatever is on top there, as a real one
// would; while the button is held it goes to the glass, which captures the pointer for a drag.
// The picker reads positions against the glass in layout pixels, so those are what it is given.
function fire(kind: string, target?: Element): Element {
  const c = client(where);
  const g = glass.getBoundingClientRect();
  const to = target ?? document.elementFromPoint(c.x, c.y) ?? document.body;
  const init = { bubbles: true, cancelable: true, composed: true, view: window, clientX: g.x + where.x, clientY: g.y + where.y, button: 0, buttons: held ? 1 : 0 };
  to.dispatchEvent(kind.startsWith("pointer") ? new PointerEvent(kind, { ...init, pointerId: 1, pointerType: "mouse", isPrimary: true }) : new MouseEvent(kind, init));
  return to;
}

let downOn: Element | null = null;
function down() {
  held = true;
  pressedAt = now;
  downOn = fire("pointerdown");
  fire("mousedown", downOn);
}
function up() {
  held = false;
  pressedAt = -1;
  const target = glass.contains(downOn) && !downOn?.closest(".window") ? glass : (downOn as Element);
  fire("pointerup", target);
  fire("mouseup", target);
  if (downOn?.isConnected) fire("click", downOn);
  downOn = null;
}
function click() {
  down();
  events.push({ frame: now + 5, run: up });
}

// A key, to whatever has the keyboard. A letter goes into a text box the way typing does.
function press(key: string) {
  const target = document.activeElement ?? document.body;
  if (!target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }))) return;
  if (key.length === 1 && target instanceof HTMLTextAreaElement) {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(target, target.value + key);
    target.dispatchEvent(new InputEvent("input", { bubbles: true, data: key, inputType: "insertText" }));
  }
  target.dispatchEvent(new KeyboardEvent("keyup", { key, bubbles: true }));
}

// ---------- the picture ----------

function pointerAt(t: number): Point {
  let p = REST;
  for (const move of moves) {
    if (t < move.t0) break;
    move.from ??= p;
    move.dest ??= move.to();
    const a = move.from;
    const b = move.dest;
    if (t >= move.t1) {
      p = b;
      continue;
    }
    // A slightly curved path: one control point off the straight line.
    const u = ease((t - move.t0) / (move.t1 - move.t0));
    const c = { x: (a.x + b.x) / 2 - (b.y - a.y) * move.bend, y: (a.y + b.y) / 2 + (b.x - a.x) * move.bend };
    return { x: (1 - u) * (1 - u) * a.x + 2 * u * (1 - u) * c.x + u * u * b.x, y: (1 - u) * (1 - u) * a.y + 2 * u * (1 - u) * c.y + u * u * b.y };
  }
  return p;
}

function cameraAt(t: number): Camera {
  let c = WIDE;
  for (const cut of cameras) {
    if (t < cut.t0) break;
    const u = ease((t - cut.t0) / (cut.t1 - cut.t0));
    c = { x: mix(c.x, cut.to.x, u), y: mix(c.y, cut.to.y, u), w: mix(c.w, cut.to.w, u) };
  }
  return c;
}

const animations = new Map<Animation, number>();
const measure = document.createElement("canvas").getContext("2d") as CanvasRenderingContext2D;

function paint(t: number) {
  const camera = cameraAt(t);
  const k = 1920 / camera.w;
  world.style.transform = `scale(${k}) translate(${-camera.x}px, ${-camera.y}px)`;
  // The pointer dips a little while its button is down.
  const dip = pressedAt < 0 ? 1 : 0.9;
  pointer.style.transform = `translate(${where.x - 3}px, ${where.y - 2}px) scale(${dip})`;
  // The bar's entrance: up 14 pixels with a little overshoot, over a third of a second.
  const u = Math.min(1, Math.max(0, (t - OPEN) / 0.36));
  const back = 1 + 2.2 * Math.pow(u - 1, 3) + 1.2 * Math.pow(u - 1, 2);
  glass.style.setProperty("--bar-in", String(Math.min(1, u * 2.5)));
  glass.style.setProperty("--bar-y", `${(1 - back) * 14}px`);
}

// After the interface has answered: the caret, and its own transitions held to the film's time.
function settle() {
  const box = document.activeElement;
  caret.hidden = !(box instanceof HTMLTextAreaElement);
  if (box instanceof HTMLTextAreaElement) {
    const corner = onStage("#comment", 0, 0)();
    measure.font = getComputedStyle(box).font;
    caret.style.transform = `translate(${corner.x + 12 + measure.measureText(box.value).width}px, ${corner.y + 11}px)`;
  }
  for (const animation of document.getAnimations()) {
    if (!animations.has(animation)) animations.set(animation, now);
    animation.pause();
    animation.currentTime = ((now - (animations.get(animation) as number)) * 1000) / FPS;
  }
}

const turn = () => new Promise<void>((done) => {
  const channel = new MessageChannel();
  channel.port1.onmessage = () => done();
  channel.port2.postMessage(0);
});

async function step(n: number): Promise<void> {
  now = n;
  const t = n / FPS;
  const next = pointerAt(t);
  const moved = next.x !== where.x || next.y !== where.y;
  where = next;
  if (moved) {
    const over = fire("pointermove", held ? glass : undefined).closest?.("button.round") ?? null;
    if (over !== hovered) {
      hovered?.removeAttribute("data-hover");
      over?.setAttribute("data-hover", "");
      hovered = over;
    }
  }
  // A click made in this frame adds its release to the list, so the list is read as it stands.
  for (let i = 0; i < events.length; i++) if (events[i].frame === n) events[i].run();
  paint(t);
  // React draws what changed in turns of its own; a few of them cover a render and its effects.
  for (let i = 0; i < 4; i++) await turn();
  settle();
  const shown = client(where);
  const speed = seen ? Math.hypot(shown.x - seen.x, shown.y - seen.y) * FPS : 0;
  if (n % 6 === 0) speeds.push(Math.round(speed));
  fastest = Math.max(fastest, speed);
  seen = shown;
}

// ---------- the stage ----------

frame.addEventListener("load", () => {
  const page = frame.contentDocument as Document;
  const origin = { x: $("app").offsetLeft + frame.offsetLeft, y: $("app").offsetTop + frame.offsetTop };
  const platform = webPlatform(
    {
      glass,
      // The page itself is not a thing to point at; everything in it is.
      elementAt: (x, y) => {
        const element = page.elementFromPoint(x - origin.x, y - origin.y);
        return element === page.documentElement || element === page.body ? null : element;
      },
      rectOf: (element) => {
        const r = element.getBoundingClientRect();
        return { x: r.x + origin.x, y: r.y + origin.y, width: r.width, height: r.height };
      },
      where: browser ? 'Google Chrome "Invoices"' : 'Northwind "Invoices"',
    },
    { tools: ["element", "area", "clip"], shortcut: "Ctrl+Shift+Space" },
  );
  // A drag captures the pointer; a scripted pointer is not one the browser knows.
  glass.setPointerCapture = () => {};
  connect(platform);
  createRoot(document.createElement("div")).render(<OnPage platform={platform} glass={glass} />);
  platform.onClose((text) => (closedWith = text));
  window.addEventListener("keydown", (event) => event.ctrlKey && event.shiftKey && event.code === "Space" && platform.open());

  const film = { fps: FPS, frames: Math.round(END * FPS), step, result: () => ({ clipboard, closedWith, shown: pasted.textContent, fastest: Math.round(fastest), speeds }) };
  Object.assign(window, { film });
  paint(0);

  if (query.has("render")) return void (document.body.dataset.ready = "1");
  // Watching: fit the picture to the window and keep time with the clock.
  const fit = () => (picture.style.transform = `scale(${Math.min(innerWidth / 1920, innerHeight / 1080)})`);
  fit();
  window.addEventListener("resize", fit);
  const started = performance.now();
  let n = 0;
  const run = async () => {
    const due = Math.min(film.frames - 1, Math.floor(((performance.now() - started) / 1000) * FPS));
    while (n <= due) await step(n++);
    if (n < film.frames) requestAnimationFrame(() => void run());
  };
  void run();
});
