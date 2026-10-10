"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { track } from "@/lib/analytics";
import { Actor } from "../demo/actor";
import { demo, open, register, replay, setPlayer, useSite } from "../demo/engine";
import Northwind from "../demo/Northwind";
import { CASES } from "./cases";
import type { CaseId, Play } from "./cases";
import Focus from "./Focus";
import Terminal from "./Terminal";
import type { Line } from "./Terminal";
import s from "./scene.module.css";

const READY = CASES.filter((c) => c.ready);
/// How long a finished example is left for the visitor before the next kind of work is shown.
const REST = 5200;

/// The first screen's picture: a desktop with the thing being built on the left, the agent's
/// terminal on the right and Clipframes over both. It plays one example for the kind of work
/// chosen under it (pick, say, paste, the agent answers, the thing changes), then it is the
/// visitor's. The bar, the outline and the comment box are the app's own, live (Live.tsx).
export default function Scene() {
  const desk = useRef<HTMLDivElement>(null);
  const page = useRef<HTMLDivElement>(null);
  const glass = useRef<HTMLDivElement>(null);
  const pointer = useRef<HTMLDivElement>(null);
  const prompt = useRef<HTMLDivElement>(null);
  const [id, setId] = useState<CaseId>("web");
  const now = useRef<CaseId>(id);
  /// The visitor chose a case, or took the scene over: it no longer moves on by itself.
  const settled = useRef(false);
  /// What the example last pasted, for when it is sent.
  const said = useRef("");
  const [lines, setLines] = useState<Line[]>([]);
  const [pasted, setPasted] = useState("");
  const [working, setWorking] = useState(false);
  const [changed, setChanged] = useState(false);
  const round = useSite((site) => site.rounds.hero);
  const taken = useSite((site) => site.taken.hero);
  const playing = useSite((site) => site.playing.hero);
  const at = CASES.find((c) => c.id === id) ?? CASES[0];

  useEffect(() => {
    if (!desk.current || !page.current || !glass.current || !pointer.current) return;
    const hand = pointer.current;
    setPlayer("hero", async (d, signal) => {
      const current = CASES.find((c) => c.id === now.current);
      if (!current?.example) return;
      const play: Play = {
        copied: () => d.round.reference,
        close: () => d.platform.done(),
        prompt: () => prompt.current as Element,
        paste: (text) => {
          said.current = text;
          setPasted(text);
        },
        send: () => {
          setLines((before) => [...before, { who: "you", text: said.current }]);
          setPasted("");
          setWorking(true);
        },
        answer: () => {
          setWorking(false);
          setLines((before) => [...before, { who: "did", text: current.did }, { who: "says", text: current.says }]);
          setChanged(true);
        },
        open: () => open(d),
      };
      const actor = new Actor(d, hand, signal);
      await current.example(actor, play);
      actor.done();
    });
    register("hero", { host: desk.current, page: page.current, glass: glass.current });
  }, []);

  /// Shows a kind of work from its start.
  const show = useCallback((next: CaseId) => {
    const d = demo("hero");
    const to = CASES.find((c) => c.id === next);
    if (!d || !to) return;
    now.current = next;
    d.dialect = to.dialect;
    setId(next);
    setLines([]);
    setPasted("");
    setWorking(false);
    setChanged(false);
    void replay(d);
  }, []);

  // The example was cut short by the visitor: the bar is up for them, and nothing is left half said.
  useEffect(() => {
    if (!taken) return;
    settled.current = true;
    setPasted("");
    setWorking(false);
    const d = demo("hero");
    if (d && !d.round.picking) open(d);
  }, [taken]);

  // A finished example rests, and then the next kind of work is shown, until the visitor steps in.
  const played = playing === false;
  useEffect(() => {
    if (!played || taken || settled.current || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const d = demo("hero");
    if (!d?.played) return;
    const timer = setTimeout(() => {
      if (settled.current || document.hidden) return;
      show(READY[(READY.findIndex((c) => c.id === now.current) + 1) % READY.length].id);
    }, REST);
    return () => clearTimeout(timer);
  }, [played, taken, id, show]);

  const choose = (next: CaseId) => {
    settled.current = true;
    track("case_chosen", { demo: next });
    show(next);
  };

  // What is in the agent's prompt: what the example pasted, or what the visitor has picked so far.
  const typed = pasted || (taken ? (round?.reference ?? "") : "");

  return (
    <figure className={s.scene} data-scene={id} data-demo="hero" data-playing={playing ? "" : undefined}>
      <div ref={desk} className={s.desk} role="group" aria-label={`A desktop with ${at.name === "Web app" ? "a web app in a browser" : "a desktop app"} on the left and ${at.agent === "claude" ? "Claude Code" : "Codex"} in a terminal on the right. Clipframes points at one thing in the app, a comment is typed, the text is pasted into the agent, and the app changes.`}>
        <div className={`${s.window} ${s.app}`} data-kind={id}>
          {id === "desktop" ? (
            <div className={s.titleBar} aria-hidden="true"><b />{at.title}<span>&#x2013;</span><span>&#x25A1;</span><span>&#x2715;</span></div>
          ) : (
            <div className={s.address} aria-hidden="true"><i /><i /><i /><span>{at.title}</span></div>
          )}
          <div ref={page} className={s.page}>
            {id === "desktop" ? (
              <Focus changed={changed} />
            ) : (
              <div className="northwind" data-changed={changed ? "" : undefined}><Northwind menu={false} simple onExport={() => {}} /></div>
            )}
          </div>
        </div>
        <Terminal look={at.agent} folder={at.folder} lines={lines} typed={typed} working={working} promptRef={prompt} />
        <div ref={glass} className={`cf ${s.glass}`} />
        <div ref={pointer} className={s.pointer} aria-hidden="true">
          <svg viewBox="0 0 24 24"><path d="M5 3l14 7.2-6 1.9-2.3 5.9z" /></svg>
        </div>
      </div>
      <figcaption className={s.cases} role="tablist" aria-label="Kinds of work">
        {CASES.map((c) => (
          <button key={c.id} role="tab" aria-selected={c.id === id} disabled={!c.ready} title={c.ready ? undefined : "Not in this preview yet"} data-case={c.id} onClick={() => choose(c.id)}>
            {c.name}
          </button>
        ))}
      </figcaption>
    </figure>
  );
}
