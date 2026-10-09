// Alternative B, "The tray": there are no rounds. A small tray sits at the edge of the screen
// for the whole work session. Holding a key turns the pointer into a pointer-at-things: every
// click drops something into the tray, and letting go gives the page back. Nothing opens on
// the page. Comments are written in the tray, now or later; things can be changed or taken
// out at any time. One button copies the lot, and what was sent stays listed underneath.

import { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { Icon } from "../../ui/icons";
import { Overlay } from "../../ui/Overlay";
import tab from "../../ui/tab.png";
import { describe, mark, outline, pointing } from "./pointing";
import type { Thing } from "./pointing";
import { ALT, lab } from "./stage";
import type { Lab } from "./stage";

const WHERE = 'Google Chrome "Invoices"';

interface Item extends Thing {
  note: string;
}
interface Batch {
  id: number;
  count: number;
  text: string;
}

/** Today's clipboard text, so the agent gets what it gets now (round.rs, reference). */
function reference(items: Item[]): string {
  const said = (item: Item) => (item.note.trim() ? `: ${item.note.trim()}` : "");
  if (items.length === 1) return `[${describe(items[0], 1)} in ${WHERE}${said(items[0])}]`;
  return [`[Clipframes: ${items.length} things in ${WHERE}]`, ...items.map((item, i) => `${i + 1}. ${describe(item, i + 1)}${said(item)}`)].join("\n");
}

function Tray({ stage }: { stage: Lab }) {
  const [items, setItems] = useState<Item[]>([]);
  const [sent, setSent] = useState<Batch[]>([]);
  const [held, setHeld] = useState(false);
  const [over, setOver] = useState(false);
  const [said, setSaid] = useState("");
  const live = useRef({ items, held });
  live.current = { items, held };
  const point = useRef<{ set(on: boolean): void } | null>(null);
  const fields = useRef(new Map<number, HTMLTextAreaElement>());
  const send = useRef<HTMLButtonElement>(null);
  // The comments the keyboard has already been through, so Enter always moves on.
  const visited = useRef(new Set<number>());
  const picked = useRef(false);

  // The next thing that still has nothing said about it, or else the button that copies.
  const onward = () => {
    const waiting = live.current.items.find((item) => item.note.trim() === "" && !visited.current.has(item.id));
    if (waiting) fields.current.get(waiting.id)?.focus();
    else send.current?.focus();
  };

  useEffect(() => {
    point.current = pointing(stage, (thing) => {
      picked.current = true;
      setItems((list) => [...list, { ...thing, note: "" }]);
      setSaid("");
    });
    const hold = (on: boolean) => {
      if (live.current.held === on) return;
      setHeld(on);
      point.current?.set(on);
      if (on) (document.activeElement as HTMLElement | null)?.blur?.();
      // Let go after pointing: the keyboard goes to the first thing with nothing said yet.
      if (!on && picked.current) window.setTimeout(onward, 60);
      picked.current = false;
    };
    stage.onKey("keydown", (event) => {
      if (event.key !== "Alt") return;
      event.preventDefault();
      hold(true);
    });
    stage.onKey("keyup", (event) => {
      if (event.key !== "Alt") return;
      event.preventDefault();
      hold(false);
    });
    window.addEventListener("blur", () => document.hasFocus() || hold(false));
    // A click on the page's own button (or the shortcut) stands in for holding the key,
    // for where a held Alt or Option does not reach the page.
    stage.onShortcut(() => hold(!live.current.held));
    stage.hint(`Hold ${ALT} and click anything`);
    stage.ready();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  // Numbers on the page only while pointing or while looking at the tray: nothing is drawn
  // over the screen the rest of the time.
  useEffect(() => mark(stage, held || over ? items : []), [items, held, over, stage]);
  useEffect(() => {
    const button = document.querySelector<HTMLElement>("[data-shortcut]");
    if (!button) return;
    button.textContent = held ? "Stop pointing" : "Start pointing";
    button.classList.toggle("on", held);
  }, [held]);
  useEffect(() => stage.hint(items.length || sent.length || held ? "" : `Hold ${ALT} and click anything`), [items.length, sent.length, held, stage]);

  const copy = (list: Item[]) => {
    const text = reference(list);
    stage.clipboard.set(text);
    setSent((all) => [{ id: Date.now(), count: list.length, text }, ...all].slice(0, 3));
    setItems([]);
    visited.current.clear();
    setSaid("Copied. Paste it into your agent.");
    (document.activeElement as HTMLElement | null)?.blur?.();
  };
  const grow = (node: HTMLTextAreaElement | null) => {
    if (!node) return;
    node.style.height = "20px";
    node.style.height = `${Math.min(80, node.scrollHeight)}px`;
  };

  const waiting = items.length;
  // While pointing, the tray folds away so that what is under it can be pointed at too.
  const folded = held || (waiting === 0 && !over && said === "");
  return (
    <div className="clipframes-web using">
      <Overlay />
      <div className={folded ? "window tray folded" : "window tray"} onPointerEnter={() => setOver(true)} onPointerLeave={() => (setOver(false), outline(stage, null))}>
        {folded ? (
          <div className={held ? "rail on" : "rail"} title={`Hold ${ALT} and click anything`}>
            <img src={tab} alt="Clipframes" draggable={false} />
            {waiting > 0 && <b className="num">{waiting}</b>}
          </div>
        ) : (
          <div className="panel">
            <header>
              <img src={tab} alt="" draggable={false} />
              <strong>{waiting === 0 ? "Nothing waiting" : waiting === 1 ? "1 thing to change" : `${waiting} things to change`}</strong>
            </header>
            {waiting === 0 && <p className="empty">{said || `Hold ${ALT} and click anything on screen. It lands here.`}</p>}
            <div className="cards">
              {items.map((item, index) => (
                <div className="card" key={item.id} onPointerEnter={() => outline(stage, item)}>
                  <b className="num">{index + 1}</b>
                  <div className="body">
                    <span className="name">{item.headline}</span>
                    <textarea
                      ref={(node) => {
                        if (node) fields.current.set(item.id, node);
                        else fields.current.delete(item.id);
                        grow(node);
                      }}
                      rows={1}
                      value={item.note}
                      placeholder="What should change?"
                      spellCheck={false}
                      aria-label={`What should change about ${item.headline}`}
                      onChange={(e) => {
                        const note = e.target.value;
                        setItems((list) => list.map((it) => (it.id === item.id ? { ...it, note } : it)));
                      }}
                      onKeyDown={(e) => {
                        if (e.key !== "Enter" || e.shiftKey || e.nativeEvent.isComposing) return;
                        e.preventDefault();
                        visited.current.add(item.id);
                        onward();
                      }}
                    />
                  </div>
                  <button className="round small" aria-label={`Take out ${index + 1}`} title="Take out" onClick={() => (outline(stage, null), setItems((list) => list.filter((it) => it.id !== item.id)))}>
                    <Icon name="close" />
                  </button>
                </div>
              ))}
            </div>
            {waiting > 0 && (
              <button ref={send} className="pill wide" onClick={() => copy(items)}>
                Copy for your agent <kbd>⏎</kbd>
              </button>
            )}
            {sent.map((batch, index) => (
              <div className="sent" key={batch.id}>
                <span>
                  {index === 0 ? "Last copied" : "Before that"}: {batch.count === 1 ? "1 thing" : `${batch.count} things`}
                </span>
                <button
                  className="quiet"
                  onClick={() => {
                    stage.clipboard.set(batch.text);
                    setSaid("Copied again.");
                  }}
                >
                  Copy again
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

const stage = await lab();
const root = document.createElement("div");
stage.glass.appendChild(root);
createRoot(root).render(<Tray stage={stage} />);
