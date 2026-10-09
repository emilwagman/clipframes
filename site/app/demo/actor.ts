// What plays a demo's example: a pointer drawn on the stage that moves, presses and types
// through the same picker a visitor uses. Nothing is faked on the app's side: the pointer's
// events go to the glass, and the comment is typed into the real box.

import type { Demo } from "./engine";
import { send } from "./touch";

interface Point {
  x: number;
  y: number;
}

class Stopped extends Error {}

const reduced = () => matchMedia("(prefers-reduced-motion: reduce)").matches;
/// Rest to rest with no jerk at either end, which is how a hand moves a pointer.
const ease = (t: number) => t * t * t * (10 - 15 * t + 6 * t * t);

export class Actor {
  /// Where the pointer is, in the glass's own pixels.
  private at: Point;
  /// With reduced motion the example is not acted out: the stage goes straight to each result.
  readonly still = reduced();

  constructor(private demo: Demo, private pointer: HTMLElement, private signal: AbortSignal) {
    const { clientWidth, clientHeight } = demo.glass;
    this.at = { x: clientWidth * 0.5, y: clientHeight * 0.72 };
    signal.addEventListener("abort", () => this.show(false));
  }

  private check(): void {
    if (this.signal.aborted) throw new Stopped();
  }

  private frame(): Promise<number> {
    return new Promise((resolve, reject) => requestAnimationFrame((now) => (this.signal.aborted ? reject(new Stopped()) : resolve(now))));
  }

  private show(on: boolean): void {
    this.pointer.dataset.on = on && !this.still ? "1" : "";
  }

  private draw(): void {
    this.pointer.style.transform = `translate(${this.at.x.toFixed(1)}px, ${this.at.y.toFixed(1)}px)`;
  }

  /// The pointer's place on the screen, which is what an event carries.
  private client(point = this.at): Point {
    const box = this.demo.glass.getBoundingClientRect();
    return { x: box.left + point.x, y: box.top + point.y };
  }

  private fire(type: "pointermove" | "pointerdown" | "pointerup"): void {
    const { x, y } = this.client();
    send(this.demo.glass, type, x, y);
  }

  /// An element of the Northwind page.
  el(selector: string): HTMLElement {
    const found = this.demo.page.querySelector<HTMLElement>(selector);
    if (!found) throw new Error(`The example looks for ${selector}, which is not in the page.`);
    return found;
  }

  has(selector: string): boolean {
    return this.demo.page.querySelector(selector) !== null;
  }

  /// A part of the app's own interface, once it is drawn.
  async ui(selector: string): Promise<HTMLElement> {
    for (let tries = 0; tries < 120; tries++) {
      const found = this.demo.glass.querySelector<HTMLElement>(`.clipframes-web:not([data-still]) ${selector}`);
      if (found) return found;
      await this.frame();
    }
    throw new Stopped();
  }

  /// The middle of an element, or a point a share of the way across it.
  point(element: Element, fx = 0.5, fy = 0.5): Point {
    const box = this.demo.glass.getBoundingClientRect();
    const r = element.getBoundingClientRect();
    return { x: r.left - box.left + r.width * fx, y: r.top - box.top + r.height * fy };
  }

  async wait(ms: number): Promise<void> {
    this.check();
    if (this.still) return void (await this.frame());
    await new Promise<void>((resolve, reject) => {
      const timer = setTimeout(resolve, ms);
      this.signal.addEventListener("abort", () => (clearTimeout(timer), reject(new Stopped())), { once: true });
    });
  }

  /// Moves the pointer, with the picker following it the whole way.
  async move(to: Point | Element, glide = true): Promise<void> {
    this.check();
    const target = to instanceof Element ? this.point(to) : to;
    const from = this.at;
    const distance = Math.hypot(target.x - from.x, target.y - from.y);
    if (!this.still && distance > 1) {
      this.show(true);
      const duration = Math.min(950, Math.max(380, distance * 1.25));
      const start = await this.frame();
      for (let now = start; now - start < duration; now = await this.frame()) {
        const t = ease((now - start) / duration);
        this.at = { x: from.x + (target.x - from.x) * t, y: from.y + (target.y - from.y) * t };
        this.draw();
        if (glide) this.fire("pointermove");
      }
    }
    this.at = target;
    this.draw();
    this.fire("pointermove");
  }

  private async press(): Promise<void> {
    this.pointer.dataset.down = "1";
    await this.wait(110);
    this.pointer.dataset.down = "";
  }

  /// Clicks where the pointer is, on the glass: a pick.
  async click(): Promise<void> {
    this.check();
    this.fire("pointerdown");
    this.fire("pointerup");
    await this.press();
  }

  /// Presses a real button: one of the bar's, or one in the page while a clip is recording.
  async push(button: HTMLElement): Promise<void> {
    await this.move(button, false);
    await this.wait(140);
    this.check();
    button.click();
    await this.press();
  }

  /// Drags from one point to another: an area.
  async drag(from: Point, to: Point): Promise<void> {
    await this.move(from, false);
    await this.wait(200);
    this.check();
    this.fire("pointerdown");
    try {
      await this.move(to);
      await this.wait(260);
    } catch (stopped) {
      // Let go where it began, which the picker takes as no area at all.
      this.at = from;
      this.fire("pointerup");
      throw stopped;
    }
    this.fire("pointerup");
  }

  /// Types into the comment box, a letter at a time.
  async type(text: string): Promise<void> {
    const box = (await this.ui("#comment")) as HTMLTextAreaElement;
    const set = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set;
    const write = (value: string) => {
      set?.call(box, value);
      box.dispatchEvent(new Event("input", { bubbles: true }));
    };
    await this.wait(320);
    if (this.still) return write(text);
    this.show(false);
    for (let i = 1; i <= text.length; i++) {
      write(text.slice(0, i));
      // A little slower after a space, like a person between words.
      await this.wait(text[i - 1] === " " ? 74 : 38);
    }
  }

  async save(): Promise<void> {
    await this.wait(380);
    await this.push(await this.ui(".note button.pill"));
  }

  /// The pointer leaves; what it did stays on the stage.
  done(): void {
    this.show(false);
  }
}

export const stopped = (error: unknown): boolean => error instanceof Stopped;
