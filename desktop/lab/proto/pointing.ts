// Pointing without tools, for the two alternative versions: the outline follows the pointer,
// a click picks the element, a drag picks an area. What is drawn goes through the app's own
// store, so the real Overlay paints it.

import type { MarkView, Rect } from "../../ui/platform";
import { useStore } from "../../ui/store";
import { selector } from "../../ui/web";
import { named } from "./names";
import type { Lab } from "./stage";

export interface Thing {
  id: number;
  kind: "element" | "area";
  /** `Button "New invoice"`, `Screenshot`. */
  headline: string;
  selector: string;
  target: Element | Rect;
}

const span = (a: { x: number; y: number }, b: { x: number; y: number }): Rect => ({ x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), width: Math.abs(a.x - b.x), height: Math.abs(a.y - b.y) });
let next = 1;

export function rectOf(stage: Lab, thing: Thing): Rect | null {
  if (!("tagName" in thing.target)) return thing.target;
  if (!thing.target.isConnected) return null;
  const rect = stage.rectOf(thing.target);
  return rect.width > 0 ? rect : null;
}

/** Numbered marks for these things, in this order. Things that share an id share a number. */
export function mark(stage: Lab, things: Thing[]): void {
  const marks: MarkView[] = [];
  const seen = new Set<number>();
  for (const thing of things) {
    if (seen.has(thing.id)) continue;
    seen.add(thing.id);
    const rect = rectOf(stage, thing);
    if (rect) marks.push({ number: seen.size, rect, kind: thing.kind });
  }
  useStore.setState({ marks });
}

export function outline(stage: Lab, thing: Thing | null): void {
  const rect = thing && rectOf(stage, thing);
  useStore.setState({ hover: rect ? { rect, label: "" } : { rect: null, label: "" } });
}

/** Wires the glass. `set(true)` gives the pointer to Clipframes, `set(false)` back to the page. */
export function pointing(stage: Lab, pick: (thing: Thing) => void): { set(on: boolean): void } {
  const glass = stage.glass;
  let on = false;
  let down: { x: number; y: number } | null = null;
  let dragging = false;
  const at = (event: MouseEvent) => {
    const box = glass.getBoundingClientRect();
    return { x: event.clientX - box.left, y: event.clientY - box.top };
  };
  const own = (event: Event) => event.target instanceof Element && event.target.closest(".window") !== null;
  const hover = (p: { x: number; y: number } | null) => {
    const element = p && stage.elementAt(p.x, p.y);
    useStore.setState({ hover: element ? { rect: stage.rectOf(element), label: named(element) } : { rect: null, label: "" } });
  };
  const area = (rect: Rect | null) => useStore.setState({ area: { rect, recording: false } });

  glass.addEventListener("pointermove", (event) => {
    if (!on) return;
    const p = at(event);
    if (down && (dragging || Math.hypot(p.x - down.x, p.y - down.y) > 8)) {
      dragging = true;
      hover(null);
      return area(span(down, p));
    }
    hover(own(event) ? null : p);
  });
  glass.addEventListener("pointerleave", () => !down && hover(null));
  glass.addEventListener("pointerdown", (event) => {
    if (!on || event.button !== 0 || own(event)) return;
    // The keyboard stays where it is: in the box being written in.
    event.preventDefault();
    down = at(event);
    glass.setPointerCapture(event.pointerId);
  });
  glass.addEventListener("pointerup", (event) => {
    if (!down) return;
    const start = down;
    const rect = span(start, at(event));
    const dragged = dragging;
    down = null;
    dragging = false;
    area(null);
    if (dragged) {
      if (rect.width >= 8 && rect.height >= 8) pick({ id: next++, kind: "area", headline: "Screenshot", selector: "", target: rect });
      return;
    }
    const element = stage.elementAt(start.x, start.y);
    if (!element) return;
    hover(null);
    pick({ id: next++, kind: "element", headline: named(element), selector: selector(element), target: element });
  });
  glass.style.pointerEvents = "none";
  return {
    set: (value) => {
      on = value;
      glass.style.pointerEvents = value ? "" : "none";
      if (!value) {
        down = null;
        dragging = false;
        hover(null);
        area(null);
      } else hover(stage.pointer);
    },
  };
}

/** The same thing, as today's clipboard text wants it. */
export const describe = (thing: Thing, number: number): string => (thing.kind === "area" ? `Screenshot (${number}.png)` : thing.selector ? `${thing.headline} (${thing.selector})` : thing.headline);
