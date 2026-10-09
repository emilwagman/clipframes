// The fix prototypes: today's interface with one thing changed at a time. The page says which
// in <body data-fix>, and ?today=1 shows the same page without the fix, to compare.
//
//   levels    arrow up and down choose the parent or the child of what is hovered
//   list      the count opens a list of the picks: edit a comment, take one back
//   through   hold Alt to use the page without ending the round; marks follow the page
//   place     the bar opens on the display being worked on and can be moved; a one-line comment box
//   after     closing leaves a way back in, and a way to get the earlier clipboard back
//   firstrun  the first time: the two permissions, then one hint per step

import { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { Note } from "../../ui/Note";
import { Overlay } from "../../ui/Overlay";
import type { Rect } from "../../ui/platform";
import { connect, useRound, useStore } from "../../ui/store";
import { Bar2 } from "./Bar2";
import { Note2 } from "./Note2";
import { ALT, lab, SHORTCUT } from "./stage";
import type { Lab } from "./stage";
import { webPlatform2 } from "./web2";
import type { Platform2, Trail } from "./web2";

type Fix = "levels" | "list" | "through" | "place" | "after" | "firstrun";
const fix = document.body.dataset.fix as Fix;
const today = new URLSearchParams(location.search).has("today");
const is = (name: Fix) => fix === name && !today;

const BAR = { width: is("through") ? 470 : 376, height: 64 };
const NOTE = is("place") ? { width: 316, height: 60 } : { width: 316, height: 172 };
const clamp = (value: number, low: number, high: number) => Math.max(low, Math.min(value, Math.max(low, high)));

function Permissions({ onDone }: { onDone: () => void }) {
  const [allowed, setAllowed] = useState({ accessibility: false, screen: false });
  const rows = [
    { key: "accessibility" as const, name: "Accessibility", why: "So Clipframes can read what you point at." },
    { key: "screen" as const, name: "Screen Recording", why: "So it can attach a picture of it." },
  ];
  useEffect(() => {
    if (!allowed.accessibility || !allowed.screen) return;
    const timer = window.setTimeout(onDone, 700);
    return () => window.clearTimeout(timer);
  }, [allowed, onDone]);
  return (
    <div className="window card permissions">
      <h2>Allow Clipframes in System Settings</h2>
      <p>macOS asks before an app may read other apps. Clipframes needs two switches turned on, once.</p>
      {rows.map((row) => (
        <div className="row" key={row.key}>
          <div>
            <strong>{row.name}</strong>
            <span>{row.why}</span>
          </div>
          {allowed[row.key] ? (
            <span className="ok" data-allowed={row.key}>
              <svg viewBox="0 0 12 12" aria-hidden="true">
                <path d="M2.5 6.5l2.4 2.4 4.6-5" />
              </svg>
              On
            </span>
          ) : (
            // The app would open the right page of System Settings and notice by itself when
            // the switch is turned on. Here the click stands in for both.
            <button className={row.key === "accessibility" || allowed.accessibility ? "pill" : "plain"} data-allow={row.key} onClick={() => window.setTimeout(() => setAllowed((a) => ({ ...a, [row.key]: true })), 500)}>
              Open Settings
            </button>
          )}
        </div>
      ))}
    </div>
  );
}

function Shell({ stage, platform }: { stage: Lab; platform: Platform2 }) {
  const round = useRound();
  const glass = stage.glass;
  const [at, setAt] = useState<Rect | null>(null);
  const [through, setThrough] = useState(false);
  const [trail, setTrail] = useState<Trail | null>(null);
  const [home, setHome] = useState({ left: 0, top: 0 });
  const [toast, setToast] = useState<{ count: number; before: string; back: boolean } | null>(null);
  const [coach, setCoach] = useState<"permissions" | "on" | "end" | "off">(is("firstrun") ? "permissions" : "off");
  const noted = useRef(false);
  const before = useRef("");
  const count = useRef(0);

  useEffect(() => platform.onNoteAt(setAt), [platform]);
  useEffect(() => platform.onThrough(setThrough), [platform]);
  useEffect(() => platform.onTrail(setTrail), [platform]);
  useEffect(() => {
    if (round.picking) count.current = round.picks.length;
    if (round.picks.length > 0 && round.noting === null) noted.current = true;
  }, [round]);

  // Where the bar opens: at the bottom of the display the pointer is on. Today it is always
  // the main display (app.rs, bar_position).
  const open = () => {
    const main = stage.displayAt(0, 0);
    const display = fix !== "place" ? { x: 0, y: 0, width: glass.clientWidth, height: glass.clientHeight } : today ? main : stage.displayAt(stage.pointer.x, stage.pointer.y);
    setHome({ left: display.x + (display.width - BAR.width) / 2, top: display.y + display.height - BAR.height - 24 });
    before.current = stage.clipboard.get();
    setToast(null);
    platform.open();
  };
  useEffect(() => {
    stage.onShortcut(() => {
      if (coach === "permissions") return;
      if (useRoundNow().picking) void platform.done();
      else open();
    });
    platform.onClose((copied) => {
      if (is("after") && copied !== "") setToast({ count: count.current, before: before.current, back: false });
      if (is("firstrun")) setCoach((c) => (c === "on" ? "end" : c));
    });
    stage.onKey("keydown", (event) => {
      const now = useRoundNow();
      if (!now.picking) return;
      if (event.key === "Escape") return void platform.escape();
      if (is("through") && event.key === "Alt") {
        event.preventDefault();
        return platform.setThrough(true);
      }
      if (is("levels") && (event.key === "ArrowUp" || event.key === "ArrowDown") && now.noting === null) {
        event.preventDefault();
        platform.step(event.key === "ArrowUp" ? 1 : -1);
      }
    });
    stage.onKey("keyup", (event) => event.key === "Alt" && platform.setThrough(false));
    window.addEventListener("blur", () => document.hasFocus() || platform.setThrough(false));
    if (!is("firstrun") && !new URLSearchParams(location.search).has("closed")) open();
    // Mounted once: the listeners read what is current when they run.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), toast.back ? 2600 : 9000);
    return () => window.clearTimeout(timer);
  }, [toast]);
  useEffect(() => {
    if (coach !== "end") return;
    const timer = window.setTimeout(() => setCoach("off"), 9000);
    return () => window.clearTimeout(timer);
  }, [coach]);
  useEffect(() => {
    stage.hint(round.picking || coach === "permissions" ? "" : `${SHORTCUT} opens Clipframes`);
  }, [round.picking, coach, stage]);

  const grip = (event: React.PointerEvent) => {
    event.preventDefault();
    const start = { x: event.clientX - home.left, y: event.clientY - home.top };
    const move = (e: PointerEvent) => setHome({ left: clamp(e.clientX - start.x, 0, glass.clientWidth - BAR.width), top: clamp(e.clientY - start.y, 0, glass.clientHeight - BAR.height) });
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  // The comment box: under the pick, or above it when there is no room, inside its display.
  const display = at && fix === "place" ? stage.displayAt(at.x + at.width / 2, at.y + at.height / 2) : { x: 0, y: 0, width: glass.clientWidth, height: glass.clientHeight };
  const note = at && {
    left: clamp(at.x, display.x + 8, display.x + display.width - NOTE.width - 8),
    top: at.y + at.height + 8 + NOTE.height <= display.y + display.height ? at.y + at.height + 8 : Math.max(display.y + 8, at.y - NOTE.height - 8),
  };
  const status = through ? { text: "Using the page", lit: true } : is("through") && round.picks.length === 0 ? { text: `Hold ${ALT} to use the page`, lit: false } : null;
  const tip =
    coach !== "on" || !round.picking
      ? null
      : round.picks.length === 0
        ? "Point at anything on screen and click it."
        : round.noting !== null && !noted.current
          ? "Say what should change, then press Enter."
          : round.noting === null
            ? "It is copied already. Paste it into Claude Code or Codex."
            : null;

  return (
    <div className={through ? "clipframes-web using" : "clipframes-web"}>
      <Overlay />
      {round.picking && trail && (
        <div className="trail" style={{ left: clamp(trail.rect.x - 2, 8, glass.clientWidth - 330), top: trail.rect.y + trail.rect.height + 30 < glass.clientHeight ? trail.rect.y + trail.rect.height + 6 : trail.rect.y - 62 }}>
          {trail.names.map((name, index) => (
            <span key={index} className={index === trail.chosen ? "on" : ""}>
              {name}
            </span>
          ))}
          <kbd>↑</kbd>
          <kbd>↓</kbd>
        </div>
      )}
      {round.picking && (
        <div className="window" style={{ ...home, ...BAR }}>
          <Bar2 platform={platform} list={is("list")} onGrip={is("place") ? grip : undefined} status={status} />
        </div>
      )}
      {round.picking && note && <div className="window" style={{ ...note, ...NOTE }}>{is("place") ? <Note2 /> : <Note />}</div>}
      {tip && (
        <div className="window tip" style={{ left: home.left + BAR.width / 2, top: home.top - 4 }}>
          {tip}
        </div>
      )}
      {coach === "permissions" && (
        <Permissions
          onDone={() => {
            setCoach("on");
            open();
          }}
        />
      )}
      {coach === "end" && !round.picking && (
        <div className="window toast">
          <span>
            Next time, press <b>{SHORTCUT}</b> wherever you are.
          </span>
        </div>
      )}
      {toast && !round.picking && (
        <div className="window toast">
          {toast.back ? (
            <span>Your earlier clipboard is back.</span>
          ) : (
            <>
              <span>
                <b className="num">{toast.count}</b> copied
              </span>
              <button
                className="plain"
                data-do="continue"
                onClick={() => {
                  setToast(null);
                  platform.reopen();
                }}
              >
                Keep picking
              </button>
              {toast.before !== "" && (
                <button
                  className="plain"
                  data-do="back"
                  onClick={() => {
                    stage.clipboard.set(toast.before);
                    setToast({ ...toast, back: true });
                  }}
                >
                  Put back what I had copied
                </button>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}

// The round as it is this instant, for listeners that outlive a render.
const useRoundNow = () => useStore.getState().round;

const stage = await lab();
const platform = webPlatform2(
  { glass: stage.glass, elementAt: stage.elementAt, rectOf: stage.rectOf, place: "Google Chrome · localhost:3000", where: 'Google Chrome "Invoices"' },
  { shortcut: SHORTCUT, levels: is("levels"), badges: is("list"), marks: fix === "through" && today ? "fixed" : "follow", windows: stage.frames.map((f) => f.contentWindow as Window) },
);
connect(platform);
platform.onRound((round) => round.reference && stage.clipboard.set(round.reference));
const root = document.createElement("div");
stage.glass.appendChild(root);
createRoot(root).render(<Shell stage={stage} platform={platform} />);
stage.ready();
