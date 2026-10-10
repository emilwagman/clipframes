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

/** A way of capturing. The bar offers the ones the platform can do. */
export type Tool = "element" | "area" | "clip";

export interface PickView {
  kind: Tool;
  /** `Button "New invoice"`, `Screenshot`, `Screen clip, 6 s`. */
  headline: string;
  /** "#new-invoice .btn.btn-primary", or empty when there is none. */
  selector: string;
  note: string;
  /** On a web page: `2nd of 2 on the page`, `under heading "…"`, as the copied text says them. The app's core words its own. */
  whereabouts?: string[];
}

/** The app or site the bar is open over, when Clipframes can tell. */
export interface PlaceView {
  /** "Google Chrome · localhost:3000", "Slack". */
  name: string;
  /** Whether Clipframes shows its tab by itself when this place is opened. */
  auto: boolean;
}

export interface RoundView {
  picking: boolean;
  tool: Tool;
  picks: PickView[];
  /** The pick whose comment box is open. */
  noting: number | null;
  /** Seconds recorded so far, while a clip is being recorded. */
  recording: number | null;
  /** What is on the clipboard now. */
  reference: string;
  /** "permission", "screen", "screen-asked", or a message, when something could not be done. */
  trouble: string | null;
  /** The shortcut as people write it, e.g. "Ctrl+Shift+Space". */
  shortcut: string;
  place: PlaceView | null;
  /** Which comment box to draw, while several are being compared: "line" is the one-line box, anything else today's. */
  noteStyle?: string;
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
  kind: Tool;
}

/** The area being dragged, or recorded. */
export interface AreaView {
  rect: Rect | null;
  recording: boolean;
}

export interface Platform {
  tools: Tool[];
  /** Whether there is a history of past rounds to open. */
  history: boolean;
  state(): Promise<RoundView>;
  onRound(listener: (round: RoundView) => void): void;
  onHover(listener: (hover: HoverView) => void): void;
  onMarks(listener: (marks: MarkView[]) => void): void;
  onArea(listener: (area: AreaView) => void): void;
  setTool(tool: Tool): Promise<void>;
  stopRecording(): Promise<void>;
  setNote(index: number, note: string): Promise<void>;
  closeNote(): Promise<void>;
  /** The one-line comment box needs a window this tall (CSS pixels) for what is typed in it. */
  resizeNote?(height: number): Promise<void>;
  removePick(index: number): Promise<void>;
  /** Turns the tab that appears by itself in this place on or off. */
  setAuto(on: boolean): Promise<void>;
  /**
   * The bar's grip was pressed: the bar follows the pointer until the button comes up. With
   * `home`, a double click: back to where it opens by default. Only where the bar is a window
   * that can be moved; without this the bar has no grip.
   */
  moveBar?(home: boolean): Promise<void>;
  openHistory(): Promise<void>;
  done(): Promise<void>;
  /** Esc, pressed while one of Clipframes' own windows has the keyboard. */
  escape(): Promise<void>;
  openPermission(kind: string): Promise<void>;
  /** Quits the app, for a permission that only takes effect in a new start. */
  quit(): Promise<void>;
}

export const EMPTY_ROUND: RoundView = { picking: false, tool: "element", picks: [], noting: null, recording: null, reference: "", trouble: null, shortcut: "", place: null };
