// The bar: what is being picked, how many so far, and Done.

import { el, icon } from "./dom";
import type { Platform, RoundView } from "./platform";

// A frame with a pointer in its corner.
const ELEMENT = "M4 3.5h9A1.5 1.5 0 0 1 14.5 5v3M4 3.5A1.5 1.5 0 0 0 2.5 5v7A1.5 1.5 0 0 0 4 13.5h4M10.5 9.5l7 2.6-3 1.2-1.2 3z";

export function mountBar(root: HTMLElement, platform: Platform): void {
  const tool = el("div", { class: "tool on" }, icon(ELEMENT), el("span", { text: "Element" }));
  const status = el("div", { class: "status" });
  const done = el("button", { class: "primary", text: "Done", click: () => void platform.done() });
  const bar = el("div", { class: "bar" }, tool, status, done);
  root.append(bar);

  const draw = (round: RoundView) => {
    bar.classList.toggle("trouble", round.trouble !== null);
    if (round.trouble === "permission") {
      status.replaceChildren(el("strong", { text: "Allow Clipframes to read other apps" }), el("span", { text: "Turn it on under Accessibility, then press " + platform.shortcut }));
      done.textContent = "Open Settings";
      done.onclick = () => void platform.openPermission();
      return;
    }
    done.onclick = null;
    if (round.trouble !== null) {
      status.replaceChildren(el("strong", { text: "Could not start" }), el("span", { text: round.trouble }));
      done.textContent = "Close";
      return;
    }
    const count = round.picks.length;
    done.textContent = count === 0 ? "Close" : "Done";
    status.replaceChildren(
      count === 0
        ? el("strong", { text: "Click anything on screen" })
        : el("strong", { text: count === 1 ? "1 thing copied" : `${count} things copied` }),
      el("span", { text: count === 0 ? "Esc to close" : "Keep clicking, or paste it to your agent" }),
    );
  };

  platform.onRound(draw);
  void platform.state().then(draw);
}
