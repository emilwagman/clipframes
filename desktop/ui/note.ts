// The comment box: what should change about the thing just picked. Optional: clicking the next
// thing, or Enter on an empty box, moves on.

import { el } from "./dom";
import type { Platform, RoundView } from "./platform";

export function mountNote(root: HTMLElement, platform: Platform): void {
  const number = el("b", { class: "num" });
  const what = el("strong", { class: "what" });
  const selector = el("code", { class: "selector" });
  const input = el("input", { type: "text", id: "comment", placeholder: "What should change?", spellcheck: "false", autocomplete: "off" });
  const keys = el("div", { class: "keys" }, el("kbd", { text: "Enter" }), el("span", { text: "save" }), el("kbd", { text: "Esc" }), el("span", { text: "skip" }), el("span", { class: "or", text: "or click the next thing" }));
  root.append(el("div", { class: "note" }, el("div", { class: "head" }, number, el("div", { class: "names" }, what, selector)), input, keys));

  let index: number | null = null;

  const save = () => (index === null || input.value.trim() === "" ? Promise.resolve() : platform.setNote(index, input.value));

  input.addEventListener("keydown", (event) => {
    if (event.key !== "Enter") return;
    void save().then(() => platform.closeNote());
  });

  platform.onRound((round: RoundView) => {
    if (round.noting === index) return;
    // A new pick while this box was open: keep what was typed for the previous one.
    void save();
    index = round.noting;
    if (index === null) return;
    const pick = round.picks[index];
    number.textContent = String(index + 1);
    what.textContent = pick?.headline ?? "";
    selector.textContent = pick?.selector ?? "";
    selector.hidden = !pick?.selector;
    input.value = pick?.note ?? "";
    input.focus();
  });
}
