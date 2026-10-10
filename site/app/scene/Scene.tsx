"use client";

import { Fragment, useCallback, useEffect, useRef, useState } from "react";
import type { PickView } from "@desktop/ui/platform";
import { track } from "@/lib/analytics";
import { Actor } from "../demo/actor";
import { demo, open, register, replay, setPlayer, useSite } from "../demo/engine";
import Northwind from "../demo/Northwind";
import { carryOut } from "./answer";
import { CASES } from "./cases";
import type { CaseId, Play } from "./cases";
import Focus from "./Focus";
import Game from "./Game";
import Motion from "./Motion";
import Proof from "./Proof";
import Terminal from "./Terminal";
import type { Line } from "./Terminal";
import s from "./scene.module.css";

/// How long a finished example is left for the visitor before the next kind of work is shown.
const REST = 5200;
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/// The first screen's picture: a desktop with the thing being built, the agent's terminal and
/// Clipframes over both. It plays one example for the kind of work chosen under it (pick, say,
/// paste, the agent answers, the thing changes), then it is the visitor's: what they pick is
/// pasted and answered too (answer.ts). The bar, the outline and the comment box are the app's
/// own, live (Live.tsx).
export default function Scene() {
  const desk = useRef<HTMLDivElement>(null);
  const page = useRef<HTMLDivElement>(null);
  const glass = useRef<HTMLDivElement>(null);
  const pointer = useRef<HTMLDivElement>(null);
  const prompt = useRef<HTMLDivElement>(null);
  const [id, setId] = useState<CaseId>("web");
  const now = useRef<CaseId>(id);
  /// Counts the times a case has been shown from its start: the thing being built is drawn anew each time.
  const [run, setRun] = useState(0);
  const runs = useRef(0);
  /// The visitor chose a case, or took the scene over: it no longer moves on by itself.
  const settled = useRef(false);
  /// What the example last pasted, for when it is sent.
  const said = useRef("");
  const [lines, setLines] = useState<Line[]>([]);
  const [pasted, setPasted] = useState("");
  const [working, setWorking] = useState(false);
  const [changed, setChanged] = useState(false);
  /// The agent has the floor: its window is in front (a phone's sheet is up) from the paste until the answer has been read.
  const [talking, setTalking] = useState(false);
  const round = useSite((site) => site.rounds.hero);
  const taken = useSite((site) => site.taken.hero);
  const playing = useSite((site) => site.playing.hero);
  const at = CASES.find((c) => c.id === id) ?? CASES[0];

  useEffect(() => {
    if (!desk.current || !page.current || !glass.current || !pointer.current) return;
    const hand = pointer.current;
    setPlayer("hero", async (d, signal) => {
      const current = CASES.find((c) => c.id === now.current);
      if (!current) return;
      const play: Play = {
        copied: () => d.round.reference,
        close: () => d.platform.done(),
        prompt: () => prompt.current as Element,
        paste: (text) => {
          said.current = text;
          setTalking(true);
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
          // Once the answer has been read the thing is in front again, where the change is.
          const run = runs.current;
          setTimeout(() => run === runs.current && setTalking(false), 1900);
        },
        open: () => {
          setTalking(false);
          open(d);
        },
      };
      const actor = new Actor(d, hand, signal);
      await current.example(actor, play);
      actor.done();
    });
    register("hero", { host: desk.current, page: page.current, glass: glass.current }, { history: false });
  }, []);

  /// Shows a kind of work from its start.
  const show = useCallback((next: CaseId) => {
    const d = demo("hero");
    const to = CASES.find((c) => c.id === next);
    if (!d || !to) return;
    now.current = next;
    runs.current += 1;
    d.dialect = to.dialect;
    d.stage.where = to.where;
    picked.current = [];
    answered.current = "";
    setId(next);
    setRun(runs.current);
    setLines([]);
    setPasted("");
    setWorking(false);
    setChanged(false);
    setTalking(false);
    void replay(d);
  }, []);

  // The example was cut short by the visitor: the bar is up for them, and nothing is left half said.
  useEffect(() => {
    if (!taken) return;
    settled.current = true;
    setPasted("");
    setWorking(false);
    setTalking(false);
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
      show(CASES[(CASES.findIndex((c) => c.id === now.current) + 1) % CASES.length].id);
    }, REST);
    return () => clearTimeout(timer);
  }, [played, taken, id, show]);

  /// The things the visitor has picked in this round, in the picks' own order.
  const picked = useRef<(Element | null)[]>([]);
  const before = useRef<PickView[]>([]);
  /// The text the agent last answered, so it answers a round once.
  const answered = useRef("");
  const picks = round?.picks;
  useEffect(() => {
    const d = demo("hero");
    const was = before.current;
    const is = picks ?? [];
    before.current = is;
    if (!d || !taken || is.length === 0) return void (picked.current = []);
    if (is.length < was.length && picked.current.length > is.length) {
      const gone = was.findIndex((pick, i) => is[i]?.headline !== pick.headline);
      picked.current = picked.current.filter((_, i) => i !== gone);
    }
    // The newest pick is the thing the picker was last asked about.
    if (picked.current.length < is.length) picked.current[is.length - 1] = is[is.length - 1].kind === "element" ? d.pointed() : null;
  }, [picks, taken]);

  // A visitor's round is theirs to paste. Once a comment is saved and they have paused, the
  // scene does it for them: the text goes into the agent's prompt, and the agent answers.
  const text = round?.reference ?? "";
  const resting = Boolean(taken && round?.picking && round.noting === null && round.recording === null && text && text !== answered.current);
  useEffect(() => {
    if (!resting) return;
    const started = runs.current;
    const here = () => started === runs.current;
    const timer = setTimeout(async () => {
      const d = demo("hero");
      if (!d || !here()) return;
      const things = d.round.picks.map((pick, i) => ({ pick, element: picked.current[i] ?? null }));
      answered.current = text;
      setTalking(true);
      setPasted(text);
      await sleep(900);
      if (!here()) return;
      setLines((lines) => [...lines, { who: "you", text }]);
      setPasted("");
      setWorking(true);
      await sleep(1200);
      if (!here()) return;
      setWorking(false);
      setLines((lines) => [...lines, ...things.map(({ pick, element }): Line => ({ who: "says", text: carryOut(pick, element) }))]);
      // As after a real paste: the round is over, and the bar is ready for the next one.
      picked.current = [];
      await d.platform.done();
      if (!here()) return;
      open(d);
      await sleep(3200);
      if (here()) setTalking(false);
    }, 1300);
    return () => clearTimeout(timer);
  }, [resting, text]);

  const choose = (next: CaseId) => {
    settled.current = true;
    track("case_chosen", { demo: next });
    show(next);
  };

  // What is in the agent's prompt: what was pasted, and until then nothing.
  const agent = at.agent === "claude" ? "Claude Code" : "Codex";

  return (
    <figure className={s.scene} data-scene={id} data-demo="hero" data-playing={playing ? "" : undefined}>
      <div
        ref={desk}
        className={s.desk}
        data-front={talking ? "agent" : "app"}
        role="group"
        aria-label={`A desktop with ${at.name.toLowerCase() === "game" ? "a game" : at.name.toLowerCase() === "motion" ? "an animation's preview" : `a ${at.name.toLowerCase()}`} and ${agent} in a terminal. Clipframes points at something in it, a comment is typed, the text is pasted into ${agent}, and the thing changes.`}
        // A press on the thing being built brings it back in front of the agent.
        onPointerDownCapture={(event) => event.isTrusted && event.target instanceof Element && !event.target.closest("[data-agent]") && setTalking(false)}
      >
        <div className={`${s.window} ${s.app}`} data-kind={at.window}>
          {at.window === "native" ? (
            <div className={s.titleBar} aria-hidden="true"><b />{at.title}<span>&#x2013;</span><span>&#x25A1;</span><span>&#x2715;</span></div>
          ) : (
            <div className={s.address} aria-hidden="true"><i /><i /><i /><span>{at.title}</span></div>
          )}
          <div ref={page} className={s.page}>
            <Fragment key={run}>
              {id === "desktop" && <Focus changed={changed} />}
              {id === "web" && <div className="northwind" data-changed={changed ? "" : undefined}><Northwind menu={false} simple onExport={() => {}} /></div>}
              {id === "motion" && <Motion changed={changed} />}
              {id === "game" && <Game changed={changed} />}
            </Fragment>
          </div>
        </div>
        <Terminal look={at.agent} folder={at.folder} before={at.before} lines={lines} typed={pasted} working={working} promptRef={prompt} />
        <div ref={glass} className={`cf ${s.glass}`} />
        <div ref={pointer} className={s.pointer} aria-hidden="true">
          <svg viewBox="0 0 24 24"><path d="M5 3l14 7.2-6 1.9-2.3 5.9z" /></svg>
        </div>
      </div>
      <figcaption className={s.under}>
        <span className={s.cases} role="tablist" aria-label="Kinds of work">
          {CASES.map((c) => (
            <button key={c.id} role="tab" aria-selected={c.id === id} data-case={c.id} onClick={() => choose(c.id)}>
              {c.name}
            </button>
          ))}
        </span>
        {id === "desktop" && <Proof />}
      </figcaption>
    </figure>
  );
}
