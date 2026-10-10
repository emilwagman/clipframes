// Touch on a demo. The picker (desktop/ui/web.ts) acts the moment a pointer goes down, which
// is right for a mouse. A finger going down may be the start of a scroll, so a finger's input
// is held back here and passed on once it is clear what it is: a tap picks the thing under
// it, and with the area or clip tool a drag that starts sideways draws the area. A drag that
// starts up or down scrolls the page as usual (touch-action: pan-y on the stage).

import type { Demo } from "./engine";
import { fresh, liveDemo } from "./engine";

export function send(glass: HTMLElement, type: "pointermove" | "pointerdown" | "pointerup", x: number, y: number): void {
  glass.dispatchEvent(new PointerEvent(type, { clientX: x, clientY: y, bubbles: true, cancelable: true, button: 0, buttons: type === "pointerup" ? 0 : 1, pointerId: 1, pointerType: "mouse", isPrimary: true }));
}

export function direct(d: Demo, take: () => void): void {
  const { host, glass } = d;
  // On the glass, and not on the bar or the comment box, which are ordinary buttons.
  const onGlass = (event: Event) => event.target instanceof Element && glass.contains(event.target) && event.target.closest(".window") === null;
  const finger = (event: PointerEvent) => event.isTrusted && event.pointerType === "touch";
  let down: { id: number; x: number; y: number } | null = null;
  let dragging = false;
  let lifted = 0;

  host.addEventListener("pointerdown", (event) => {
    if (!finger(event) || !onGlass(event)) return;
    event.stopPropagation();
    down = { id: event.pointerId, x: event.clientX, y: event.clientY };
    dragging = false;
  }, true);

  host.addEventListener("pointermove", (event) => {
    if (!finger(event) || !down || event.pointerId !== down.id) return;
    event.stopPropagation();
    if (d.round.tool === "element" || d.round.recording !== null || liveDemo() !== d) return;
    if (!dragging) {
      if (Math.hypot(event.clientX - down.x, event.clientY - down.y) < 10) return;
      dragging = true;
      take();
      fresh(d);
      send(glass, "pointerdown", down.x, down.y);
    }
    send(glass, "pointermove", event.clientX, event.clientY);
  }, true);

  host.addEventListener("pointerup", (event) => {
    if (!finger(event)) return;
    lifted = event.timeStamp;
    if (!down || event.pointerId !== down.id) return;
    event.stopPropagation();
    if (dragging) send(glass, "pointerup", event.clientX, event.clientY);
    down = null;
    dragging = false;
  }, true);

  // The browser took the gesture for a scroll: an area that was begun is given up.
  host.addEventListener("pointercancel", (event) => {
    if (!finger(event) || !down) return;
    if (dragging) send(glass, "pointerup", down.x, down.y);
    down = null;
    dragging = false;
  }, true);

  host.addEventListener("click", (event) => {
    // A click that follows a finger lifting is a tap.
    if (!event.isTrusted || event.timeStamp - lifted > 700) return;
    const wasLive = liveDemo() === d;
    take();
    // The first tap on a demo that was showing its still copy only wakes it.
    if (!wasLive || !onGlass(event) || !d.round.picking || d.round.tool !== "element") return;
    fresh(d);
    send(glass, "pointerdown", event.clientX, event.clientY);
    send(glass, "pointerup", event.clientX, event.clientY);
  }, true);
}
