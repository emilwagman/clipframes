// The bar: what is being picked, what has been picked so far, and Done.

import { el, icon } from "./dom";
import type { Platform, RoundView } from "./platform";

import type { Tool } from "./platform";

const TOOLS: Record<Tool, { label: string; icon: string }> = {
  // A frame with a pointer in its corner.
  element: { label: "Element", icon: "M4 3.5h9A1.5 1.5 0 0 1 14.5 5v3M4 3.5A1.5 1.5 0 0 0 2.5 5v7A1.5 1.5 0 0 0 4 13.5h4M10.5 9.5l7 2.6-3 1.2-1.2 3z" },
  // The four corners of a dragged area.
  area: { label: "Area", icon: "M3 7V4.5A1.5 1.5 0 0 1 4.5 3H7M13 3h2.5A1.5 1.5 0 0 1 17 4.5V7M17 13v2.5a1.5 1.5 0 0 1-1.5 1.5H13M7 17H4.5A1.5 1.5 0 0 1 3 15.5V13" },
  // A screen with a record dot.
  clip: { label: "Clip", icon: "M4 4h12a1.5 1.5 0 0 1 1.5 1.5v9A1.5 1.5 0 0 1 16 16H4a1.5 1.5 0 0 1-1.5-1.5v-9A1.5 1.5 0 0 1 4 4zM10 7.6a2.4 2.4 0 1 0 0 4.8 2.4 2.4 0 0 0 0-4.8z" },
};
// A clock.
const HISTORY = "M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM10 6v4.2l2.8 1.6";

export function mountBar(root: HTMLElement, platform: Platform): void {
  // One tool is a plain label; several are a row to choose from.
  const tiles = platform.tools.map((tool, i) => el("div", { class: i === 0 ? "tool on" : "tool" }, icon(TOOLS[tool].icon), el("span", { text: TOOLS[tool].label })));
  const mode = el("div", { class: platform.tools.length > 1 ? "tools" : "mode" }, ...tiles);
  const status = el("div", { class: "status" });
  const picks = el("div", { class: "picks" });
  const esc = el("div", { class: "keys" }, el("kbd", { text: "Esc" }));
  const done = el("button", { class: "primary", text: "Done", click: () => void platform.done() });
  const history = el("button", { class: "icon", title: "History", hidden: !platform.history }, icon(HISTORY));
  const bar = el("div", { class: "bar" }, mode, status, picks, esc, history, done);
  root.append(bar);

  const draw = (round: RoundView) => {
    bar.classList.toggle("trouble", round.trouble !== null);
    picks.replaceChildren();
    if (round.trouble === "permission") {
      status.replaceChildren(el("strong", { text: "Allow Clipframes to read other apps" }), el("span", { text: "Turn it on under Accessibility, then press " + round.shortcut }));
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
    bar.classList.toggle("empty", count === 0);
    done.textContent = count === 0 ? "Close" : "Done";
    status.replaceChildren(
      count === 0 ? el("strong", { text: "Click anything on screen" }) : el("strong", { text: count === 1 ? "1 thing copied" : `${count} things copied` }),
      el("span", { text: count === 0 ? `${round.shortcut} opens and closes this` : "Keep clicking, or paste it to your agent" }),
    );
    // The last few picks by number and name. Only some looks show them.
    picks.replaceChildren(...round.picks.slice(-3).map((pick, i) => el("span", { class: "chip" }, el("b", { text: String(Math.max(0, count - 3) + i + 1) }), el("span", { text: pick.headline.replace(/^\w+ /, "").replace(/^"|"$/g, "") }))));
  };

  platform.onRound(draw);
  void platform.state().then(draw);
}
