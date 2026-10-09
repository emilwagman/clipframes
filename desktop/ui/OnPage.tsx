// The whole interface over a stage, for a web page: the overlay on the glass, the bar at the
// bottom of it, and the comment box under whatever was just picked.

import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { Bar } from "./Bar";
import { Note } from "./Note";
import { Overlay } from "./Overlay";
import type { Rect } from "./platform";
import { useRound } from "./store";
import type { WebPlatform } from "./web";

// The sizes the app gives the two windows, shadow room included.
const BAR = { width: 376, height: 64 };
const NOTE = { width: 316, height: 172 };

export function OnPage({ platform, glass }: { platform: WebPlatform; glass: HTMLElement }) {
  const round = useRound();
  const [at, setAt] = useState<Rect | null>(null);
  useEffect(() => platform.onNoteAt(setAt), [platform]);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => event.key === "Escape" && round.picking && void platform.escape();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [platform, round.picking]);

  if (!round.picking) return null;
  // Under the pick, or above it when there is no room, and always inside the glass.
  const box = { width: glass.clientWidth, height: glass.clientHeight };
  const note = at && {
    left: Math.max(8, Math.min(at.x, box.width - NOTE.width - 8)),
    top: at.y + at.height + 8 + NOTE.height <= box.height ? at.y + at.height + 8 : Math.max(8, at.y - NOTE.height - 8),
  };
  return createPortal(
    <div className="clipframes-web">
      <Overlay />
      <div className="window" style={{ left: (box.width - BAR.width) / 2, bottom: 24, ...BAR }}>
        <Bar />
      </div>
      {note && (
        <div className="window" style={{ ...note, ...NOTE }}>
          <Note />
        </div>
      )}
    </div>,
    glass,
  );
}
