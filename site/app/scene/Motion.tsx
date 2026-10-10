// A made-up motion project in its preview: a card that comes into frame, over a timeline. As it
// starts, the card overshoots where it lands, which is the fault the example's clip shows.

import "./motion.css";

/// `changed` is the scene after the agent has done what the example asked: the card eases into place.
export default function Motion({ changed = false }: { changed?: boolean }) {
  return (
    <div className="motionapp" id="studio" data-changed={changed ? "" : undefined}>
      <div id="preview">
        <div id="card"><i /><b /><b /></div>
      </div>
      <div id="timeline">
        <div id="transport"><i />00:01 / 00:04<span>60 fps</span></div>
        <div id="tracks">
          <div id="track-card"><span>Card</span><b /></div>
          <div id="track-title"><span>Title</span><b /></div>
          <div id="track-logo"><span>Logo</span><b /></div>
          <i id="playhead" />
        </div>
      </div>
    </div>
  );
}
