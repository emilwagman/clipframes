// The bar: which tool is on, what has been picked so far, and Done.

import { Icon } from "./icons";
import type { Tool } from "./platform";
import { usePlatform, useRound } from "./store";

const LABELS: Record<Tool, string> = { element: "Element", area: "Area", clip: "Clip" };
const HINTS: Record<Tool, string> = { element: "Click anything on screen", area: "Drag over an area", clip: "Drag over an area to record" };

function clock(seconds: number): string {
  return `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;
}

export function Bar() {
  const platform = usePlatform();
  const round = useRound();
  const count = round.picks.length;

  if (round.trouble !== null) {
    const permission = round.trouble === "permission" || round.trouble === "screen";
    return (
      <div className="bar trouble">
        <div className="status">
          <strong>{round.trouble === "permission" ? "Allow Clipframes to read other apps" : round.trouble === "screen" ? "Allow Clipframes to capture the screen" : "Could not start"}</strong>
          <span>{round.trouble === "permission" ? `Turn it on under Accessibility, then press ${round.shortcut}` : round.trouble === "screen" ? "Turn it on under Screen Recording, then try again" : round.trouble}</span>
        </div>
        <button className="primary" onClick={() => void (permission ? platform.openPermission(round.trouble as string) : platform.done())}>
          {permission ? "Open Settings" : "Close"}
        </button>
      </div>
    );
  }

  if (round.recording !== null) {
    return (
      <div className="bar recording">
        <span className="dot" />
        <div className="status">
          <strong>Recording {clock(round.recording)}</strong>
          <span>Use the app as usual. Clicks are noted.</span>
        </div>
        <button className="primary" onClick={() => void platform.stopRecording()}>
          Stop
        </button>
      </div>
    );
  }

  return (
    <div className="bar">
      <div className="tools" role="tablist">
        {platform.tools.map((tool) => (
          <button key={tool} role="tab" aria-selected={round.tool === tool} className={round.tool === tool ? "tool on" : "tool"} onClick={() => void platform.setTool(tool)}>
            <Icon name={tool} />
            <span>{LABELS[tool]}</span>
          </button>
        ))}
      </div>
      <div className="status">
        <strong>{count === 0 ? HINTS[round.tool] : count === 1 ? "1 thing copied" : `${count} things copied`}</strong>
        <span>{count === 0 ? `Esc or ${round.shortcut} closes this` : "Keep going, or paste it to your agent"}</span>
      </div>
      {round.place && (
        <button className={round.place.auto ? "ghost on" : "ghost"} title={round.place.auto ? `Clipframes appears by itself in ${round.place.name}. Click to stop.` : `Show Clipframes by itself in ${round.place.name}`} onClick={() => void platform.setAuto(!round.place?.auto)}>
          <Icon name="pin" />
        </button>
      )}
      {platform.history && (
        <button className="ghost" title="History" onClick={() => void platform.openHistory()}>
          <Icon name="history" />
        </button>
      )}
      <button className="primary" onClick={() => void platform.done()}>
        {count === 0 ? "Close" : "Done"}
      </button>
    </div>
  );
}
