// What the first screen's scene can show: one kind of work per case, each with the thing being
// built, the agent it is being built with, and one short example of Clipframes between the two.

import { reference } from "@desktop/ui/web";
import type { RoundView } from "@desktop/ui/platform";
import type { Actor } from "../demo/actor";
import type { Dialect } from "../demo/engine";

export type CaseId = "web" | "desktop" | "motion" | "game";
export type AgentLook = "claude" | "codex";

/// One exchange in the agent's window: what was asked, the tool call it made, and what it said.
export interface Exchange {
  ask: string;
  did: string;
  says: string;
}

/// What the example does in the scene besides picking, which is the actor's own.
export interface Play {
  /// What is on the clipboard now.
  copied(): string;
  /// Closes the bar, as Esc does.
  close(): Promise<void>;
  prompt(): Element;
  paste(text: string): void;
  send(): void;
  answer(): void;
  /// The bar comes back up, for the visitor.
  open(): void;
}

export interface Case {
  id: CaseId;
  name: string;
  agent: AgentLook;
  /// Where the agent is working, as its window says.
  folder: string;
  /// The window the thing is in: a browser's, with an address, or a desktop app's, with a title.
  window: "browser" | "native";
  title: string;
  /// What the copied text calls the place, as the app reads it from the window.
  where: string;
  /// What was said in the agent's window before the example begins: the work was already going on.
  before: Exchange;
  /// The agent's answer to the example.
  did: string;
  says: string;
  dialect: Dialect | null;
  example: (a: Actor, play: Play) => Promise<void>;
}

/// From the saved comment to the changed thing: the part every example ends with.
async function hand(a: Actor, play: Play): Promise<void> {
  await a.save();
  await a.wait(480);
  const text = play.copied();
  await play.close();
  await a.wait(350);
  await a.tap(play.prompt());
  play.paste(text);
  await a.wait(900);
  play.send();
  a.done();
  await a.wait(1300);
  play.answer();
  await a.wait(2600);
  play.open();
}

/// A window that is not a web page, in the words the app uses for one on Windows: parts are
/// named by kind and automation id, and there is no page to say where in it they are.
function native(app: string, names: [RegExp, string][]): Dialect {
  const label = (said: string) => names.reduce((now, [from, to]) => now.replace(from, to), said);
  return {
    label,
    text: (round: RoundView) => reference(round.picks.map((pick) => ({ ...pick, headline: label(pick.headline), selector: pick.selector.replace(/^#(\S+).*$/, "id=$1"), whereabouts: [] })), app),
  };
}

export const CASES: Case[] = [
  {
    id: "web", name: "Web app", agent: "claude", folder: "~/northwind", window: "browser", title: "localhost:3000", where: 'Google Chrome "Invoices"', dialect: null,
    before: { ask: "add a search field to the invoices page", did: "Update(src/components/InvoiceHeader.tsx)", says: "Added a search field next to Export." },
    did: "Update(src/components/InvoiceHeader.tsx)", says: "The New invoice button is black now.",
    example: async (a, play) => {
      await a.wait(700);
      await a.move(a.el("#paid-total strong"));
      await a.wait(420);
      await a.move(a.el("#export"));
      await a.wait(360);
      await a.move(a.el("#new-invoice"));
      await a.wait(520);
      await a.click();
      await a.type("make this black");
      await hand(a, play);
    },
  },
  {
    id: "desktop", name: "Desktop app", agent: "codex", folder: "~/focus", window: "native", title: "Focus", where: "Focus",
    dialect: native("Focus", [[/^Label /, "CheckBox "], [/^Text field /, "Edit "], [/^Item /, "ListItem "]]),
    before: { ask: "add a Skip break button after Reset", did: "Edited MainWindow.xaml (+1 -0)", says: "Skip break is next to Reset." },
    did: "Edited MainWindow.xaml (+1 -1)", says: "The Start button is green now.",
    example: async (a, play) => {
      await a.wait(700);
      await a.move(a.el("#TimeLeft"));
      await a.wait(420);
      await a.move(a.el("#ResetButton"));
      await a.wait(360);
      await a.move(a.el("#StartButton"));
      await a.wait(520);
      await a.click();
      await a.type("make this green");
      await hand(a, play);
    },
  },
  {
    // Something that moves is shown with a clip: the agent gets the frames in order.
    id: "motion", name: "Motion", agent: "claude", folder: "~/launch-film", window: "browser", title: "localhost:3001", where: 'Google Chrome "Launch film"', dialect: null,
    before: { ask: "bring the card in from below", did: "Update(src/scenes/Card.tsx)", says: "The card rises into place with a spring." },
    did: "Update(src/scenes/Card.tsx)", says: "The card eases out now. The overshoot is gone.",
    example: async (a, play) => {
      await a.wait(700);
      await a.push(await a.ui('.bar button[aria-label="Record an area"]'));
      const stage = a.el("#preview");
      await a.drag(a.point(stage, 0.07, 0.08), a.point(stage, 0.93, 0.92));
      // The clip is as long as the wait and the walk to Stop: about two seconds.
      await a.wait(a.still ? 0 : 1150);
      await a.push(await a.ui(".bar button.pill"));
      await a.type("the card overshoots, ease it out");
      await hand(a, play);
    },
  },
  {
    // A game draws its own picture, so there are no parts to name: an area shows the place.
    id: "game", name: "Game", agent: "codex", folder: "~/hopper", window: "native", title: "Hopper", where: "Hopper",
    dialect: native("Hopper", [[/^Group$/, "Canvas"]]),
    before: { ask: "show the score in the top left", did: "Edited src/hud.ts (+6 -0)", says: "The score is drawn in the top left." },
    did: "Edited src/hud.ts (+2 -2)", says: "The score sits under the health bar now.",
    example: async (a, play) => {
      await a.wait(700);
      await a.push(await a.ui('.bar button[aria-label="Screenshot an area"]'));
      const view = a.el("#view");
      const from = a.point(view, 0, 0);
      await a.drag({ x: from.x + 6, y: from.y + 6 }, { x: from.x + 214, y: from.y + 64 });
      await a.type("the health bar overlaps the score");
      await hand(a, play);
    },
  },
];
