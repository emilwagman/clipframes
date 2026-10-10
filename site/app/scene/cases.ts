// What the first screen's scene can show: one kind of work per case, each with the thing being
// built, the agent it is being built with, and one short example of Clipframes between the two.

import { reference } from "@desktop/ui/web";
import type { RoundView } from "@desktop/ui/platform";
import type { Actor } from "../demo/actor";
import type { Dialect } from "../demo/engine";

export type CaseId = "web" | "desktop" | "motion" | "game";
export type AgentLook = "claude" | "codex";

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
  /// False for a case that is named in the row and not built yet.
  ready: boolean;
  agent: AgentLook;
  /// Where the agent is working, as its window says.
  folder: string;
  /// The window's name: an address for a browser, a title for a desktop app.
  title: string;
  /// The agent's answer, as two lines of its window: what it did, and what it says.
  did: string;
  says: string;
  dialect: Dialect | null;
  example?: (a: Actor, play: Play) => Promise<void>;
}

/// From the pick to the changed thing: the part every example ends with.
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

/// A native window's parts as the app names them on Windows: by kind and automation id.
const native: Dialect = {
  label: (label) => label.replace(/^Label /, "CheckBox ").replace(/^Text field /, "Edit ").replace(/^Item /, "ListItem "),
  text: (round: RoundView) => reference(round.picks.map((pick) => ({ ...pick, headline: native.label(pick.headline), selector: pick.selector.replace(/^#(\S+).*$/, "id=$1"), whereabouts: [] })), "Focus"),
};

export const CASES: Case[] = [
  {
    id: "web", name: "Web app", ready: true, agent: "claude", folder: "~/northwind", title: "localhost:3000", dialect: null,
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
    id: "desktop", name: "Desktop app", ready: true, agent: "codex", folder: "~/focus", title: "Focus", dialect: native,
    did: "Edited MainWindow.xaml", says: "The Start button is green now.",
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
  { id: "motion", name: "Motion", ready: false, agent: "claude", folder: "~/launch-film", title: "Preview", dialect: null, did: "", says: "" },
  { id: "game", name: "Game", ready: false, agent: "codex", folder: "~/hopper", title: "Hopper", dialect: null, did: "", says: "" },
];
