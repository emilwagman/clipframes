// The example each demo plays by itself until the visitor takes over. Each is one short thing
// a person would do with Clipframes, done at the pace a person does it.

import type { Actor } from "./actor";
import type { DemoId } from "./engine";

export const EXAMPLES: Partial<Record<DemoId, (a: Actor) => Promise<void>>> = {
  // Point at one thing and say what should change.
  hero: async (a) => {
    await a.wait(900);
    await a.move(a.el("#paid-total"));
    await a.wait(520);
    await a.move(a.el("#invoice-search"));
    await a.wait(420);
    await a.move(a.el("#new-invoice"));
    await a.wait(620);
    await a.click();
    await a.type("make this green");
  },

  // Three things and a comment on each. The last is one of four that read the same, which the
  // copied text tells apart. The pointer ends on a fourth thing.
  picks: async (a) => {
    await a.wait(600);
    await a.move(a.el("#new-invoice"));
    await a.wait(380);
    await a.click();
    await a.type("make this green");
    await a.save();
    await a.move(a.el("#overdue-total strong"));
    await a.wait(380);
    await a.click();
    await a.type("too alarming, use the normal text colour");
    await a.save();
    await a.move(a.el("#invoice-table tbody tr:nth-child(4) .badge"));
    await a.wait(380);
    await a.click();
    await a.type("make this one grey");
    await a.save();
    // Its corner, which is the card itself and not the words in it.
    await a.move(a.point(a.el("#outstanding-total"), 0.86, 0.26));
  },

  // An area around the three totals.
  area: async (a) => {
    await a.wait(600);
    const first = a.point(a.el("#paid-total"), 0, 0);
    const last = a.point(a.el("#overdue-total"), 1, 1);
    await a.drag({ x: first.x - 10, y: first.y - 10 }, { x: last.x + 10, y: last.y + 10 });
    await a.type("put more space between these");
  },

  // A clip of the Export menu opening in the wrong place.
  clip: async (a) => {
    await a.wait(600);
    const top = a.el(".top");
    const first = a.point(top, 0, 0);
    const last = a.point(top, 1, 1);
    await a.drag({ x: first.x - 12, y: first.y - 10 }, { x: last.x + 12, y: last.y + 136 });
    await a.wait(700);
    await a.push(a.el("#export"));
    // The clip is as long as the wait: a little over three seconds from the drag to Stop.
    await a.wait(a.still ? 0 : 1500);
    await a.push(await a.ui(".bar button.pill"));
    await a.type("the menu opens far from the button");
  },
};
