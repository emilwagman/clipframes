// The overlay: a highlight that follows the pointer, the area being dragged or recorded, and a
// number on each thing picked. It only paints; nothing here handles input.
//
// The highlight moves at the rate of the mouse, so it is positioned straight from the store
// without a render: what is drawn is always the newest answer, never a queued one.

import { useEffect, useRef } from "react";
import type { Rect } from "./platform";
import { useStore } from "./store";

function place(node: HTMLElement, r: Rect): void {
  node.style.transform = `translate(${Math.round(r.x)}px, ${Math.round(r.y)}px)`;
  node.style.width = `${Math.round(r.width)}px`;
  node.style.height = `${Math.round(r.height)}px`;
}

const box = (r: Rect) => ({ transform: `translate(${Math.round(r.x)}px, ${Math.round(r.y)}px)`, width: Math.round(r.width), height: Math.round(r.height) });

export function Overlay() {
  const highlight = useRef<HTMLDivElement>(null);
  const label = useRef<HTMLDivElement>(null);
  const marks = useStore((s) => s.marks);
  const area = useStore((s) => s.area);

  useEffect(() => {
    const draw = ({ hover }: { hover: { rect: Rect | null; label: string } }) => {
      const node = highlight.current;
      if (!node || !label.current) return;
      node.hidden = hover.rect === null;
      if (hover.rect === null) return;
      place(node, hover.rect);
      label.current.textContent = hover.label;
      // The label sits above the element, or inside it when there is no room above.
      label.current.classList.toggle("inside", hover.rect.y < 32);
      // A container the size of the page gets an outline only: a tint that large hides the page.
      node.classList.toggle("big", hover.rect.width * hover.rect.height > window.innerWidth * window.innerHeight * 0.2);
    };
    draw(useStore.getState());
    return useStore.subscribe(draw);
  }, []);

  return (
    <>
      <div className="marks">
        {marks.map((mark) => (
          <div key={mark.number} className={`mark ${mark.kind}`} style={box(mark.rect)}>
            <b>{mark.number}</b>
          </div>
        ))}
      </div>
      {area.rect && (
        <div className={area.recording ? "area recording" : "area"} style={box(area.rect)}>
          {!area.recording && <span className="size">{`${Math.round(area.rect.width)} × ${Math.round(area.rect.height)}`}</span>}
        </div>
      )}
      <div ref={highlight} className="highlight" hidden>
        <div ref={label} className="label" />
      </div>
    </>
  );
}
