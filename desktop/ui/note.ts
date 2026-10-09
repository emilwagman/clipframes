// The comment box: what should change about the thing just picked. Optional: clicking the next
// thing, or Enter on an empty box, moves on.

import { el } from "./dom";
import type { Platform, RoundView } from "./platform";

export function mountNote(root: HTMLElement, platform: Platform): void {
  const what = el("div", { class: "what" });
  const input = el("input", { type: "text", placeholder: "What should change? (optional)", spellcheck: "false", autocomplete: "off" });
  const hint = el("div", { class: "hint", text: "Enter to save · Esc to skip · or click the next thing" });
  root.append(el("div", { class: "note" }, what, input, hint));

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
    what.textContent = `${index + 1}. ${pick?.headline ?? ""}`;
    input.value = pick?.note ?? "";
    input.focus();
  });
}
