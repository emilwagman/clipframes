// What the scene's agent does with a visitor's own pick. It is a made-up agent in a made-up
// app, so it only claims what it has really done: a few plain requests are carried out on the
// thing that was picked, and for anything else it says what it got and what it can do here.

import type { PickView } from "@desktop/ui/platform";

const COLOURS: Record<string, string> = { green: "#1f9d55", black: "#17181c", red: "#d6333a", blue: "#2563eb", orange: "#f76808", grey: "#8a8d97", gray: "#8a8d97", white: "#ffffff", yellow: "#e6b800", purple: "#7c3aed", pink: "#e2468a" };

/// Said when a request is not one of the few the scene can carry out.
export const ONLY = 'This demo only acts on a colour, "bigger", "smaller", "hide" and "rename to".';

const light = (hex: string): boolean => {
  const n = parseInt(hex.slice(1), 16);
  return ((n >> 16) * 299 + ((n >> 8) & 255) * 587 + (n & 255) * 114) / 1000 > 170;
};

/// Marks a thing for a moment, so a change to it is seen.
function mark(element: HTMLElement): void {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  element.animate([{ boxShadow: "0 0 0 7px rgba(247, 104, 8, 0.24)" }, { boxShadow: "0 0 0 7px rgba(247, 104, 8, 0.24)", offset: 0.35 }, { boxShadow: "0 0 0 7px transparent" }], { duration: 1600, easing: "cubic-bezier(0.4, 0, 0.2, 1)" });
}

/// Carries out what a comment asks of the thing it is about, where that is one of the few
/// things understood here, and says what was done in one line.
export function carryOut(pick: PickView, element: Element | null): string {
  const name = pick.headline;
  if (pick.kind === "clip") return `Got the clip. ${ONLY}`;
  if (pick.kind === "area") return `Got the screenshot. ${ONLY}`;
  const target = element instanceof HTMLElement && !(element instanceof HTMLCanvasElement) && element.isConnected ? element : null;
  const note = pick.note.trim();
  if (!target || !note) return `Got ${name}. ${ONLY}`;
  mark(target);
  const says = /(?:rename(?: (?:it|this))? to|call (?:it|this)|should say|says?)\s+["“']?(.+?)["”']?\.?$/i.exec(note);
  if (says && target.childElementCount === 0) {
    target.textContent = says[1];
    return `${name} now says "${says[1]}".`;
  }
  if (/\b(hide|remove|delete)\b/i.test(note)) {
    target.style.visibility = "hidden";
    return `${name} is hidden now.`;
  }
  const colour = Object.keys(COLOURS).find((word) => new RegExp(`\\b${word}\\b`, "i").test(note));
  if (colour) {
    const filled = !/rgba?\(\s*\d+,\s*\d+,\s*\d+,\s*0\s*\)|transparent/.test(getComputedStyle(target).backgroundColor);
    if (filled && /^(BUTTON|A)$/.test(target.tagName) || filled && target.classList.contains("badge")) {
      target.style.backgroundColor = COLOURS[colour];
      target.style.borderColor = COLOURS[colour];
      target.style.color = light(COLOURS[colour]) ? "#17181c" : "#ffffff";
    } else {
      target.style.color = COLOURS[colour];
    }
    return `${name} is ${colour === "gray" ? "grey" : colour} now.`;
  }
  const bigger = /\b(bigger|larger)\b/i.test(note);
  if (bigger || /\bsmaller\b/i.test(note)) {
    const by = bigger ? 1.25 : 0.82;
    const style = getComputedStyle(target);
    target.style.fontSize = `${(parseFloat(style.fontSize) * by).toFixed(1)}px`;
    if (target.tagName === "BUTTON") target.style.padding = `${(parseFloat(style.paddingTop) * by).toFixed(1)}px ${(parseFloat(style.paddingLeft) * by).toFixed(1)}px`;
    return `${name} is ${bigger ? "bigger" : "smaller"} now.`;
  }
  return `Got ${name}. ${ONLY}`;
}
