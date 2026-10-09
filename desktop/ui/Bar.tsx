// The bar: which tool is on, how much has been copied so far, and a way out. One row of icons;
// the words live in tooltips and in the label that follows the pointer.

import { Icon } from "./icons";
import type { Tool } from "./platform";
import { usePlatform, useRound } from "./store";

const NAMES: Record<Tool, string> = { element: "Point at an element", area: "Screenshot an area", clip: "Record an area" };
const HINTS: Record<Tool, string> = { element: "Click anything", area: "Drag an area", clip: "Drag to record" };

function clock(seconds: number): string {
  return `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;
}

export function Bar() {
  const platform = usePlatform();
  const round = useRound();
  const count = round.picks.length;

  if (round.trouble !== null) {
    const permission = round.trouble === "permission" || round.trouble === "screen";
    const message = round.trouble === "permission" ? "Needs Accessibility access" : round.trouble === "screen" ? "Needs Screen Recording access" : round.trouble;
    return (
      <div className="bar wide">
        <span className="status" title={message}>
          {message}
        </span>
        <button className="pill" onClick={() => void (permission ? platform.openPermission(round.trouble as string) : platform.done())}>
          {permission ? "Open Settings" : "Close"}
        </button>
      </div>
    );
  }

  if (round.recording !== null) {
    return (
      <div className="bar wide">
        <span className="dot" />
        <span className="status">
          <b className="time">{clock(round.recording)}</b> Recording, use the app as usual
        </span>
        <button className="pill" onClick={() => void platform.stopRecording()}>
          Stop
        </button>
      </div>
    );
  }

  return (
    <div className="bar">
      <div className="tools" role="tablist">
        {platform.tools.map((tool) => (
          <button key={tool} role="tab" aria-selected={round.tool === tool} aria-label={NAMES[tool]} title={NAMES[tool]} className={round.tool === tool ? "round on" : "round"} onClick={() => void platform.setTool(tool)}>
            <Icon name={tool} />
          </button>
        ))}
      </div>
      <i className="rule" />
      <span className="status">
        {count === 0 ? (
          HINTS[round.tool]
        ) : (
          <>
            <b className="num">{count}</b> copied
          </>
        )}
      </span>
      {round.place && (
        <button className={round.place.auto ? "round lit" : "round"} aria-label="Appear here by itself" title={round.place.auto ? `Clipframes appears by itself in ${round.place.name}. Click to stop.` : `Show Clipframes by itself in ${round.place.name}`} onClick={() => void platform.setAuto(!round.place?.auto)}>
          <Icon name="pin" />
        </button>
      )}
      {platform.history && (
        <button className="round" aria-label="History" title="History" onClick={() => void platform.openHistory()}>
          <Icon name="history" />
        </button>
      )}
      <i className="rule" />
      <button className="round" aria-label="Close" title={`Close (Esc or ${round.shortcut})`} onClick={() => void platform.done()}>
        <Icon name="close" />
      </button>
    </div>
  );
}
