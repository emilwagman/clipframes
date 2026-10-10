"use client";

import { useEffect, useRef } from "react";
import { track } from "@/lib/analytics";
import { useSite } from "./engine";
import type { DemoId } from "./engine";
import s from "./prompt.module.css";

/// An agent's prompt with what a demo has copied pasted into it: the text is the demo's own
/// (desktop/ui/web.ts writes it), and it changes as things are picked. `example` is what the
/// demo's example ends up with, shown faintly until the demo has a round of its own, so the
/// prompt is never empty.
/// `docked` is the prompt as the foot of its demo's window, which is how the first screen has
/// it: it is not there until something is picked, and then it is as tall as its text.
export default function Prompt({ id, example = "", docked = false }: { id: DemoId; example?: string; docked?: boolean }) {
  const box = useRef<HTMLDivElement & HTMLElement>(null);
  const round = useSite((site) => site.rounds[id]);
  const taken = useSite((site) => site.taken[id]);
  const own = useSite((site) => site.clipboard);
  const text = round?.reference ?? "";
  const by = taken ? "visitor" : "example";

  // Counted once for the example and once for the visitor: the text was there, and on screen.
  const counted = useRef<Record<string, boolean>>({});
  const seen = useRef(false);
  useEffect(() => {
    const count = () => {
      if (!seen.current || !text || counted.current[by]) return;
      counted.current[by] = true;
      track("reference_shown", { demo: id, by });
    };
    count();
    if (!box.current || typeof IntersectionObserver === "undefined") return;
    const watch = new IntersectionObserver(([entry]) => {
      seen.current = entry.isIntersecting;
      count();
    }, { threshold: 0.6 });
    watch.observe(box.current);
    return () => watch.disconnect();
  }, [id, text, by]);

  // The foot keeps its last text while it closes, so what closes is not an empty strip.
  const last = useRef("");
  if (text) last.current = text;

  if (docked) {
    return (
      <div ref={box} className={s.foot} data-prompt={id} data-open={text ? "" : undefined}>
        <div className={s.footIn}>
          <div className={s.title}>Pasted into your agent</div>
          <pre className={s.text} data-copied={id} data-state={text ? (taken ? "own" : "example") : "empty"} aria-live="polite" onCopy={() => track("reference_copied")}>
            <span className={s.caret} aria-hidden="true">&gt; </span>
            <span key={round?.picks.length} className={s.line}>{text || last.current}</span>
          </pre>
        </div>
      </div>
    );
  }

  return (
    <figure ref={box} className={s.prompt} data-prompt={id}>
      <div className={s.window}>
        <div className={s.title}>Pasted into your agent</div>
        <pre className={s.text} data-copied={id} data-state={text ? (taken ? "own" : "example") : "empty"} aria-live="polite" onCopy={() => track("reference_copied")}>
          <span className={s.caret} aria-hidden="true">&gt; </span>
          {text ? <span key={round?.picks.length} className={s.line}>{text}</span> : <span className={s.ghost}>{example}</span>}
        </pre>
      </div>
      <figcaption>
        {!text
          ? "This is the text one pick puts on your clipboard, pasted into an agent."
          : taken && own === text
            ? "This is what you picked. It is on your own clipboard now, and you can paste it anywhere to check."
            : taken
              ? "This is what you picked, as your agent would get it."
              : "This is what the example picked, as your agent would get it. The app puts it on your clipboard after every pick."}
      </figcaption>
    </figure>
  );
}
