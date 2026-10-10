// Which one, and under what heading: what tells apart two things on a page that read the same.
// The app's core finds and words these for real apps (src-tauri/src/element/locate.rs); this
// is the same for a web page, word for word, so the website's demos copy the same text.

/** What the look through a page needs from an element. A DOM element has all of it. */
export interface Walked {
  tagName: string;
  children: ArrayLike<Walked>;
  textContent: string | null;
  getAttribute(name: string): string | null;
}

/**
 * How far the look may go before it gives up and says nothing. Smaller than the app's own
 * budget: there the look is taken after the pick is on screen, here the click waits for it.
 */
export const BUDGET = { nodes: 3000, ms: 80 };

/** "1st", "2nd", "3rd", "4th", "11th", "21st". */
export function ordinal(n: number): string {
  const ending = n % 100 >= 11 && n % 100 <= 13 ? "th" : n % 10 === 1 ? "st" : n % 10 === 2 ? "nd" : n % 10 === 3 ? "rd" : "th";
  return `${n}${ending}`;
}

/** A heading's text on one line, cut to about sixty characters. */
export function headingText(text: string): string {
  const line = text.split(/\s+/).filter(Boolean).join(" ");
  const letters = [...line];
  return letters.length <= 60 ? line : `${letters.slice(0, 59).join("").trimEnd()}…`;
}

const isHeading = (e: Walked) => /^h[1-6]$/i.test(e.tagName) || e.getAttribute("role") === "heading";

/**
 * `["2nd of 2 on the page", 'under heading "Try it on your own app."']` for `target`, looking
 * through everything under `root` in document order. `same` says whether an element reads the
 * same as the target; pass null for a target with no name, which is not counted. Empty when
 * there is nothing to say, or the page is larger than the budget.
 */
export function whereabouts<E extends Walked>(root: E, target: E, same: ((element: E) => boolean) | null, budget = BUDGET, now: () => number = () => performance.now()): string[] {
  const started = now();
  let [count, nth, visited] = [0, 0, 0];
  let before: string | null = null;
  let own: string | null = null;
  let targetDepth = -1;
  let left = false;
  const stack: [E, number][] = [[root, 0]];
  while (stack.length > 0) {
    const [node, depth] = stack.pop() as [E, number];
    if (visited >= budget.nodes || now() - started > budget.ms) return [];
    visited += 1;
    if (targetDepth >= 0 && depth <= targetDepth) left = true;
    const inside = targetDepth >= 0 && !left;
    if (same?.(node)) count += 1;
    if (node === target && targetDepth < 0) {
      targetDepth = depth;
      nth = same ? count : 0;
    } else if (isHeading(node)) {
      const text = headingText(node.textContent ?? "");
      if (text && targetDepth < 0) before = text;
      else if (text && inside && own === null) own = text;
    }
    // Without a name there is nothing to count: the heading is all that is looked for.
    if (!same && targetDepth >= 0 && (before !== null || own !== null || left)) break;
    for (let i = node.children.length - 1; i >= 0; i--) stack.push([node.children[i] as E, depth + 1]);
  }
  if (targetDepth < 0) return [];
  const said: string[] = [];
  if (nth > 0 && count > 1) said.push(`${ordinal(nth)} of ${count} on the page`);
  // A heading that was picked is under no heading: its own words already say where it is.
  if (!isHeading(target)) {
    if (before !== null) said.push(`under heading "${before}"`);
    else if (own !== null) said.push(`with heading "${own}"`);
  }
  return said;
}
