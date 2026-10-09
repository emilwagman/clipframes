// The overlay: a highlight that follows the pointer and a number on each thing picked.
// It only paints. Nothing here handles input, and nothing animates the highlight's position,
// so what is drawn is always the newest answer.

import { el } from "./dom";
import type { Platform, Rect } from "./platform";

function place(node: HTMLElement, r: Rect): void {
  node.style.transform = `translate(${Math.round(r.x)}px, ${Math.round(r.y)}px)`;
  node.style.width = `${Math.round(r.width)}px`;
  node.style.height = `${Math.round(r.height)}px`;
}

export function mountOverlay(root: HTMLElement, platform: Platform): void {
  const label = el("div", { class: "label" });
  const highlight = el("div", { class: "highlight", hidden: true }, label);
  const marks = el("div", { class: "marks" });
  root.append(marks, highlight);

  platform.onHover((hover) => {
    highlight.hidden = hover.rect === null;
    if (hover.rect === null) return;
    place(highlight, hover.rect);
    label.textContent = hover.label;
    // The label sits above the element, or inside it when there is no room above.
    label.classList.toggle("inside", hover.rect.y < 28);
  });

  platform.onMarks((list) => {
    marks.replaceChildren(
      ...list.map((mark) => {
        const node = el("div", { class: "mark" }, el("b", { text: String(mark.number) }));
        place(node, mark.rect);
        return node;
      }),
    );
  });
}
