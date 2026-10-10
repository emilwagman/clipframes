// The bar: which tool is on, how much has been copied so far, and a way out. One row of icons;
// the words live in tooltips and in the label that follows the pointer.

import { Icon } from "./icons";
import { useComment } from "./Note";
import type { Tool } from "./platform";
import { usePlatform, useRound } from "./store";

const NAMES: Record<Tool, string> = { element: "Point at an element", area: "Screenshot an area", clip: "Record an area" };
const HINTS: Record<Tool, string> = { element: "Click anything", area: "Drag an area", clip: "Drag to record" };

function clock(seconds: number): string {
  return `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, "0")}`;
}

/** Six dots at the start of the bar to move it by, so what is under it can be picked. */
function Grip() {
  const platform = usePlatform();
  if (!platform.moveBar) return null;
  return (
    <span
      className="grip"
      title="Drag to move. Double click to put it back."
      // Decided on the press: the system takes the pointer for the drag, so no click follows it.
      onMouseDown={(event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        void platform.moveBar?.(event.detail > 1);
      }}
    >
      <i />
      <i />
      <i />
      <i />
      <i />
      <i />
    </span>
  );
}

export function Bar() {
  const platform = usePlatform();
  const round = useRound();
  const count = round.picks.length;
  // The comment typed here instead of in a box at the pick ("dock"): nothing covers the page.
  const comment = useComment<HTMLInputElement>(round.noteStyle === "dock");

  if (round.trouble !== null) {
    // "screen-asked": Screen Recording is still refused after a visit to System Settings.
    const screen = round.trouble === "screen" || round.trouble === "screen-asked";
    const permission = round.trouble === "permission" || screen;
    const message = round.trouble === "permission" ? "Needs Accessibility access" : round.trouble === "screen" ? "Needs Screen Recording" : round.trouble === "screen-asked" ? "Quit and reopen Clipframes" : round.trouble;
    const more = screen ? "Screenshots and clips need Screen Recording access. After you turn it on, Clipframes may need to be quit and opened again." : message;
    return (
      <div className={round.picking ? "bar wide back" : "bar wide"}>
        <Grip />
        {/* The round is still on when only pictures are the trouble: pointing at elements works. */}
        {round.picking && (
          <button className="round" aria-label={NAMES.element} title="Back to pointing at elements" onClick={() => void platform.setTool("element")}>
            <Icon name="element" />
          </button>
        )}
        <span className="status" title={more}>
          {message}
        </span>
        {round.trouble === "screen-asked" ? (
          // The line says what to do, and the button does it.
          <button className="pill" title={more} onClick={() => void platform.quit()}>
            Quit Clipframes
          </button>
        ) : (
          <button className="pill" title={screen ? more : undefined} onClick={() => void (permission ? platform.openPermission(round.trouble as string) : platform.done())}>
            {permission ? "Open Settings" : "Close"}
          </button>
        )}
      </div>
    );
  }

  if (round.recording !== null) {
    return (
      <div className="bar wide">
        <Grip />
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
      <Grip />
      <div className="tools" role="tablist">
        {platform.tools.map((tool) => (
          <button key={tool} role="tab" aria-selected={round.tool === tool} aria-label={NAMES[tool]} title={NAMES[tool]} className={round.tool === tool ? "round on" : "round"} onClick={() => void platform.setTool(tool)}>
            <Icon name={tool} />
          </button>
        ))}
      </div>
      <i className="rule" />
      {comment.index !== null && comment.pick && (
        <>
          <b className="num">{comment.index + 1}</b>
          <span className="what" title={comment.pick.headline}>
            {comment.pick.headline}
          </span>
          <input ref={comment.input} className="say" value={comment.text} placeholder="What should change?" spellCheck={false} autoComplete="off" onChange={(e) => comment.write(e.target.value)} onKeyDown={comment.onKeyDown(false)} />
          <button className="round small" aria-label="Remove this pick" title="Remove this pick" onClick={() => void comment.remove()}>
            <Icon name="trash" />
          </button>
        </>
      )}
      <span className="status" hidden={comment.index !== null}>
        {count === 0 ? (
          HINTS[round.tool]
        ) : (
          <>
            <b className="num">{count}</b> copied
          </>
        )}
      </span>
      {round.place && comment.index === null && (
        <button className={round.place.auto ? "round lit" : "round"} aria-label="Appear here by itself" title={round.place.auto ? `Clipframes appears by itself in ${round.place.name}. Click to stop.` : `Show Clipframes by itself in ${round.place.name}`} onClick={() => void platform.setAuto(!round.place?.auto)}>
          <Icon name="pin" />
        </button>
      )}
      {platform.history && comment.index === null && (
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
