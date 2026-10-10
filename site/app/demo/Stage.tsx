"use client";

import { useEffect, useRef, useState } from "react";
import tab from "@desktop/ui/tab.png";
import type { Tool } from "@desktop/ui/platform";
import { Actor } from "./actor";
import { demo, open, register, replay, setPlayer, take, useSite } from "./engine";
import type { DemoId } from "./engine";
import { EXAMPLES } from "./examples";
import Northwind from "./Northwind";
import s from "./stage.module.css";

interface Props {
  id: DemoId;
  /// The tool the bar starts with.
  tool?: Tool;
  /// False for a stage that starts with Clipframes closed and only its tab showing.
  opens?: boolean;
  /// What the picture shows, for someone who cannot see it.
  label: string;
  /// What a visitor can do here, with a mouse and with a finger.
  hint: string;
  touchHint: string;
  /// How tall the stage is: the hero's follows the hero's layout, and the others are fixed.
  size?: "hero" | "tall" | "side" | "short";
  /// Says what a visitor can do above the stage, where it is read before the stage is: the hero.
  invite?: boolean;
}

/// One demo: the Northwind page, the glass the app's interface is drawn on, and the pointer
/// that plays the example. The interface itself is mounted by Live, for whichever demo is live.
export default function Stage({ id, tool, opens = true, label, hint, touchHint, size = "side", invite = false }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const page = useRef<HTMLDivElement>(null);
  const glass = useRef<HTMLDivElement>(null);
  const pointer = useRef<HTMLDivElement>(null);
  const round = useSite((site) => site.rounds[id]);
  const playing = useSite((site) => site.playing[id]);
  const [menu, setMenu] = useState(false);

  useEffect(() => {
    if (!host.current || !page.current || !glass.current || !pointer.current) return;
    const hand = pointer.current;
    const example = EXAMPLES[id];
    if (example) {
      setPlayer(id, async (d, signal) => {
        const actor = new Actor(d, hand, signal);
        await example(actor);
        actor.done();
      });
    }
    register(id, { host: host.current, page: page.current, glass: glass.current }, { tool, opens });
  }, [id, tool, opens]);

  // Every time the example starts, the page is as it was: the Export menu is closed.
  useEffect(() => {
    if (playing) setMenu(false);
  }, [playing]);

  const closed = round !== undefined && !round.picking;
  const tabOn = closed && round.place?.auto !== false;
  const again = () => {
    const d = demo(id);
    if (!d) return;
    setMenu(false);
    replay(d);
  };
  const reopen = (auto: boolean) => {
    const d = demo(id);
    if (!d) return;
    take(d);
    if (auto) void d.platform.setAuto(true);
    else open(d);
  };

  const says = (
    <span data-hint="">
      <span className={s.mouse}>{hint}</span>
      <span className={s.finger}>{touchHint}</span>
    </span>
  );

  return (
    <figure className={`${s.stage} ${s[size]}`} data-demo={id}>
      {invite && <p className={s.invite}>{says}</p>}
      <div className={s.frame}>
        <div className={s.address} aria-hidden="true"><i /><i /><i /><span>localhost:3000</span></div>
        <div ref={host} className={s.view} role="group" aria-label={label}>
          <div ref={page} className="northwind"><Northwind menu={menu} onExport={() => setMenu((on) => !on)} /></div>
          <div ref={glass} className={`cf ${s.glass}`} />
          {tabOn && (
            <div className={`cf ${s.tab}`}>
              <button className="tabmark" title="Open Clipframes" onClick={() => reopen(false)}>
                <img src={tab.src} alt="Open Clipframes" draggable={false} />
              </button>
            </div>
          )}
          <div ref={pointer} className={s.pointer} aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M5 3l14 7.2-6 1.9-2.3 5.9z" /></svg>
          </div>
        </div>
      </div>
      <figcaption className={s.caption}>
        {closed && !tabOn ? (
          <span>
            The tab is off for this page now. <button onClick={() => reopen(true)}>Turn it back on</button>
          </span>
        ) : invite ? <span /> : says}
        {EXAMPLES[id] && <button onClick={again} data-off={playing ? "" : undefined}>Play the example again</button>}
      </figcaption>
    </figure>
  );
}
