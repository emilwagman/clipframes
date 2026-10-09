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

export interface Platform {
  /** How the shortcut is written on this system, e.g. "⌃⇧Space". */
  shortcut: string;
  state(): Promise<RoundView>;
  onRound(listener: (round: RoundView) => void): void;
  onHover(listener: (hover: HoverView) => void): void;
  onMarks(listener: (marks: MarkView[]) => void): void;
  setNote(index: number, note: string): Promise<void>;
  closeNote(): Promise<void>;
  removePick(index: number): Promise<void>;
  done(): Promise<void>;
  openPermission(): Promise<void>;
}
