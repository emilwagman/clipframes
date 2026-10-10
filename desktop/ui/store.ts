// One store for what the core says is happening. Components read from it; nothing in the
// interface keeps its own copy of the round.

import { create } from "zustand";
import { EMPTY_ROUND } from "./platform";
import type { AreaView, HoverView, MarkView, Platform, RoundView } from "./platform";

interface State {
  platform: Platform | null;
  round: RoundView;
  hover: HoverView;
  marks: MarkView[];
  area: AreaView;
  /** The pointer is over the comment box that lets it through: it is drawn faint. */
  faint: boolean;
}

export const useStore = create<State>(() => ({
  platform: null,
  round: EMPTY_ROUND,
  hover: { rect: null, label: "" },
  marks: [],
  area: { rect: null, recording: false },
  faint: false,
}));

/** Feeds the store from a platform. Call once per window, before rendering. */
export function connect(platform: Platform): void {
  useStore.setState({ platform });
  platform.onRound((round) => useStore.setState({ round }));
  platform.onHover((hover) => useStore.setState({ hover }));
  platform.onMarks((marks) => useStore.setState({ marks }));
  platform.onArea((area) => useStore.setState({ area }));
  platform.onFaint?.((faint) => useStore.setState({ faint }));
  void platform.state().then((round) => useStore.setState({ round }));
}

export const usePlatform = (): Platform => useStore((s) => s.platform) as Platform;
export const useRound = (): RoundView => useStore((s) => s.round);
