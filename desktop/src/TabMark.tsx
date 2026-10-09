// The tab, where it is a window of the app's own (on Windows the core draws it itself).

import { invoke } from "@tauri-apps/api/core";
import mark from "../ui/tab.png";

export function TabMark() {
  return (
    <button className="tabmark" title="Open Clipframes" onClick={() => void invoke("tab_open")}>
      <img src={mark} alt="Clipframes" draggable={false} />
    </button>
  );
}
