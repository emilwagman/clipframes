// A copy of ui/Bar.tsx with what the fix prototypes add: the count opens a list of the picks
// so far, a grip moves the bar, and the status can say that the page is being used.

import { useState } from "react";
import { Icon } from "../../ui/icons";
import type { Tool } from "../../ui/platform";
import { useRound, useStore } from "../../ui/store";
import type { Platform2 } from "./web2";

const NAMES: Record<Tool, string> = { element: "Point at an element", area: "Screenshot an area", clip: "Record an area" };
const HINTS: Record<Tool, string> = { element: "Click anything", area: "Drag an area", clip: "Drag to record" };

function clock(seconds: number): string {
  return `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;
}

export interface BarOptions {
  list?: boolean;
  /** Called with the pointer event that starts a drag of the bar. */
  onGrip?: (event: React.PointerEvent) => void;
  /** Said in place of the hint or the count. */
  status?: { text: string; lit: boolean } | null;
}

export function Bar2({ platform, list, onGrip, status }: BarOptions & { platform: Platform2 }) {
  const round = useRound();
  const marks = useStore((s) => s.marks);
  const [open, setOpen] = useState(false);
  const count = round.picks.length;

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

  const point = (index: number | null) => {
    const mark = index === null ? undefined : marks.find((m) => m.number === index + 1);
    useStore.setState({ hover: mark ? { rect: mark.rect, label: "" } : { rect: null, label: "" } });
  };
  const showList = list && open && count > 0;

  return (
    <>
      {showList && (
        <div className="picks" onPointerLeave={() => point(null)}>
          {round.picks.map((pick, index) => (
            <div key={index} className={round.noting === index ? "pick on" : "pick"} onPointerEnter={() => point(index)}>
              <b className="num">{index + 1}</b>
              <button className="words" title="Edit the comment" onClick={() => platform.openNote(index)}>
                <span className="name">{pick.headline}</span>
                <span className={pick.note ? "said" : "said none"}>{pick.note || "No comment"}</span>
              </button>
              <button className="round small" aria-label={`Remove ${index + 1}`} title="Remove" onClick={() => {
                  point(null);
                  void platform.removePick(index);
                }}>
                <Icon name="close" />
              </button>
            </div>
          ))}
        </div>
      )}
      <div className={onGrip ? "bar gripped" : "bar"}>
        {onGrip && (
          <span className="grip" title="Drag to move" onPointerDown={onGrip}>
            <i /><i /><i /><i /><i /><i />
          </span>
        )}
        <div className="tools" role="tablist">
          {platform.tools.map((tool) => (
            <button key={tool} role="tab" aria-selected={round.tool === tool} aria-label={NAMES[tool]} title={NAMES[tool]} className={round.tool === tool ? "round on" : "round"} onClick={() => void platform.setTool(tool)}>
              <Icon name={tool} />
            </button>
          ))}
        </div>
        <i className="rule" />
        {status ? (
          <span className={status.lit ? "status lit" : "status"}>{status.text}</span>
        ) : count === 0 ? (
          <span className="status">{HINTS[round.tool]}</span>
        ) : list ? (
          <button className={open ? "status count open" : "status count"} aria-expanded={open} title="See and change what you picked" onClick={() => setOpen(!open)}>
            <b className="num">{count}</b> copied
            <svg className="chev" viewBox="0 0 10 10" aria-hidden="true">
              <path d="M2 6.5l3-3 3 3" />
            </svg>
          </button>
        ) : (
          <span className="status">
            <b className="num">{count}</b> copied
          </span>
        )}
        {round.place && !onGrip && (
          <button className={round.place.auto ? "round lit" : "round"} aria-label="Appear here by itself" title={round.place.auto ? `Clipframes appears by itself in ${round.place.name}. Click to stop.` : `Show Clipframes by itself in ${round.place.name}`} onClick={() => void platform.setAuto(!round.place?.auto)}>
            <Icon name="pin" />
          </button>
        )}
        <i className="rule" />
        <button className="round" aria-label="Close" title={`Close (Esc or ${round.shortcut})`} onClick={() => void platform.done()}>
          <Icon name="close" />
        </button>
      </div>
    </>
  );
}
