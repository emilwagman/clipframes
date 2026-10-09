// The stage every prototype stands on: one or more pages in frames, a sheet of glass over
// them, a stand-in for the global shortcut, and a stand-in for the system clipboard that
// shows what is on it.

import "../../ui/style.css";
import "./proto.css";
import type { Rect } from "../../ui/platform";

const mac = /Mac/.test(navigator.platform);
/** The keys as each system writes them. */
export const SHORTCUT = mac ? "⌃⇧Space" : "Ctrl+Shift+Space";
export const ALT = mac ? "Option" : "Alt";

export interface Lab {
  glass: HTMLElement;
  frames: HTMLIFrameElement[];
  /** The page element under a point of the glass, whichever frame it is in. */
  elementAt(x: number, y: number): Element | null;
  /** Where a page element is, in the glass's pixels. */
  rectOf(element: Element): Rect;
  /** The frame under a point of the glass, as a rect in the glass's pixels. */
  displayAt(x: number, y: number): Rect;
  /** Where the pointer was last seen, in the glass's pixels. */
  pointer: { x: number; y: number };
  /** Ctrl+Shift+Space, wherever the keyboard is. */
  onShortcut(listener: () => void): void;
  /** A key event from the host page or any of the framed pages. */
  onKey(type: "keydown" | "keyup", listener: (event: KeyboardEvent) => void): void;
  clipboard: { get(): string; set(text: string): void };
  /** The words on the chip that says how to start. Empty hides it. */
  hint(text: string): void;
  ready(): void;
}

const loaded = (frame: HTMLIFrameElement) =>
  new Promise<void>((done) => {
    if (frame.contentDocument?.readyState === "complete" && frame.contentDocument.body?.childElementCount) return done();
    frame.addEventListener("load", () => done(), { once: true });
  });

export async function lab(): Promise<Lab> {
  const glass = document.getElementById("glass") as HTMLElement;
  const frames = [...document.querySelectorAll<HTMLIFrameElement>("iframe.page")];
  await Promise.all(frames.map(loaded));
  const open = document.getElementById("open") as HTMLButtonElement;
  const clip = document.getElementById("clip") as HTMLElement;
  const docs = [document, ...frames.map((f) => f.contentDocument as Document)];

  const offset = (frame: Element) => {
    const g = glass.getBoundingClientRect();
    const r = frame.getBoundingClientRect();
    return { x: r.left - g.left, y: r.top - g.top, width: r.width, height: r.height };
  };
  const frameAt = (x: number, y: number) => frames.find((f) => ((o) => x >= o.x && x < o.x + o.width && y >= o.y && y < o.y + o.height)(offset(f)));

  const pointer = { x: glass.clientWidth / 2, y: glass.clientHeight / 2 };
  document.addEventListener("pointermove", (e) => {
    const g = glass.getBoundingClientRect();
    pointer.x = e.clientX - g.left;
    pointer.y = e.clientY - g.top;
  });
  frames.forEach((f) =>
    f.contentDocument?.addEventListener("pointermove", (e) => {
      const o = offset(f);
      pointer.x = o.x + e.clientX;
      pointer.y = o.y + e.clientY;
    }),
  );

  const shortcut: (() => void)[] = [];
  docs.forEach((d) =>
    d.addEventListener("keydown", (e) => {
      if (e.code !== "Space" || !e.ctrlKey || !e.shiftKey) return;
      e.preventDefault();
      shortcut.forEach((f) => f());
    }),
  );
  open.addEventListener("click", () => shortcut.forEach((f) => f()));

  let held = clip.dataset.start ?? "";
  const show = () => {
    clip.hidden = held === "";
    (clip.querySelector("pre") as HTMLElement).textContent = held;
  };
  show();

  return {
    glass,
    frames,
    elementAt: (x, y) => {
      const f = frameAt(x, y);
      if (!f) return null;
      const o = offset(f);
      return (f.contentDocument as Document).elementFromPoint(x - o.x, y - o.y);
    },
    rectOf: (element) => {
      const r = element.getBoundingClientRect();
      const f = element.ownerDocument.defaultView?.frameElement;
      const o = f ? offset(f) : { x: 0, y: 0 };
      return { x: o.x + r.x, y: o.y + r.y, width: r.width, height: r.height };
    },
    displayAt: (x, y) => {
      const f = frameAt(x, y) ?? frames[0];
      return offset(f);
    },
    pointer,
    onShortcut: (f) => void shortcut.push(f),
    onKey: (type, f) => docs.forEach((d) => d.addEventListener(type, f as EventListener)),
    clipboard: {
      get: () => held,
      set: (text) => {
        held = text;
        show();
        void navigator.clipboard?.writeText(text).catch(() => {});
      },
    },
    hint: (text) => {
      open.hidden = text === "";
      open.textContent = text;
    },
    ready: () => void (document.body.dataset.ready = "1"),
  };
}
