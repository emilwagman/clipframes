// The door between the interface and whatever it runs on.
//
// The desktop app implements this with Tauri commands and events (src/tauri.ts). The website
// implements it over its own page, so the bar people try there is this exact code.

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface PickView {
  headline: string;
  /** "#new-invoice .btn.btn-primary", or empty when the element has no id or class. */
  selector: string;
  note: string;
}

export interface RoundView {
  picking: boolean;
  picks: PickView[];
  /** The pick whose comment box is open. */
  noting: number | null;
  /** What is on the clipboard now. */
  reference: string;
  /** "permission", or a message, when the round could not start. */
  trouble: string | null;
  /** The shortcut as people write it, e.g. "Ctrl+Shift+Space". */
  shortcut: string;
}

/** The highlight under the pointer, in the overlay's own pixels. */
export interface HoverView {
  rect: Rect | null;
  label: string;
}

/** A numbered outline on something already picked. */
export interface MarkView {
  number: number;
  rect: Rect;
}

/** A way of capturing. The bar offers the ones the platform can do. */
export type Tool = "element" | "area" | "clip";

export interface Platform {
  tools: Tool[];
  /** Whether there is a history of past rounds to open. */
  history: boolean;
  state(): Promise<RoundView>;
  onRound(listener: (round: RoundView) => void): void;
  onHover(listener: (hover: HoverView) => void): void;
  onMarks(listener: (marks: MarkView[]) => void): void;
  setNote(index: number, note: string): Promise<void>;
  closeNote(): Promise<void>;
  removePick(index: number): Promise<void>;
  done(): Promise<void>;
  /** Esc, pressed while one of Clipframes' own windows has the keyboard. */
  escape(): Promise<void>;
  openPermission(): Promise<void>;
}
