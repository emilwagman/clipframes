// How the prototypes name what is pointed at. ui/web.ts joins the text of a row's cells with
// nothing between them; this reads the text the way it is laid out, with spaces.

import { headline } from "../../ui/web";

export function named(element: Element): string {
  const plain = headline(element);
  const quote = plain.indexOf(' "');
  if (quote < 0 || element.getAttribute("aria-label") || element.getAttribute("alt") || element.getAttribute("placeholder")) return plain;
  const text = ((element as HTMLElement).innerText ?? element.textContent ?? "").replace(/\s+/g, " ").trim();
  if (text === "") return plain;
  return `${plain.slice(0, quote)} "${text.length > 40 ? `${text.slice(0, 39)}…` : text}"`;
}

/** The fewest words that tell one element from its parents: its id, or its class, or its kind. */
export function brief(element: Element): string {
  if (element.id) return `#${element.id}`;
  if (element.classList.length) return `.${element.classList[0]}`;
  return headline(element).replace(/ ".*/, "");
}
