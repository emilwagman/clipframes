"use client";

import { track } from "@/lib/analytics";
import { useSite } from "./engine";
import s from "./copied.module.css";

/// The text the example round copies, shown until the demo above has a round of its own.
const EXAMPLE = `[Clipframes: 3 things in Google Chrome "Invoices"]
1. Button "New invoice" (#new-invoice .btn.btn-primary), under heading "Invoices": make this green
2. Text "$3,120" (#overdue-total), under heading "Invoices": too alarming, use the normal text colour
3. Text "Paid" (#invoice-table .badge.paid), 2nd of 4 on the page, under heading "Invoices": make this one grey`;

/// A round with one of each kind, in the app's own words (desktop/src-tauri/src/round.rs).
const MIXED = `[Clipframes: 3 things in Google Chrome "Invoices"]
1. Button "New invoice" (#new-invoice .btn.btn-primary), under heading "Invoices": make this green
2. Screenshot (2.png): the table is cramped
3. Screen clip, 6 s, 24 frames (3/)`;

/// What the demo above it has copied so far, as the picks are made.
export function Copied() {
  const round = useSite((site) => site.rounds.picks);
  const own = useSite((site) => site.clipboard);
  const playing = useSite((site) => site.playing.picks);
  const taken = useSite((site) => site.taken.picks);
  // Before the example has played, and when it was cut short, the block shows where it ends up.
  const text = round?.reference || (taken || playing ? "" : EXAMPLE);
  return (
    <div className={s.copied}>
      <pre className={s.text} data-copied="picks" aria-live="polite" onCopy={() => track("reference_copied")}>{text || <span className={s.none}>Nothing is picked yet.</span>}</pre>
      <p className={s.under}>
        {own !== null && own === text
          ? "This text is on your own clipboard now. Paste it anywhere to check."
          : "Everything you pick is on your clipboard straight away. Keep going, or paste."}
      </p>
    </div>
  );
}

/// An agent's prompt with the newest thing the visitor copied on this page pasted into it.
export function Pasted() {
  const latest = useSite((site) => site.latest);
  return (
    <figure className={s.prompt}>
      <div className={s.window}>
        <pre className={s.text} data-copied="latest" onCopy={() => track("reference_copied")}><span className={s.caret} aria-hidden="true">&gt; </span>{latest || MIXED}</pre>
      </div>
      <figcaption>
        {latest
          ? "This is what you last picked on this page."
          : "This is a round with an element, a screenshot and a clip. Pick something in a demo above and it shows up here."}
      </figcaption>
    </figure>
  );
}
