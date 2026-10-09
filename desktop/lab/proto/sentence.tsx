// Alternative A, "Write and point": there is no bar, no tool and no comment box. The shortcut
// opens one box to write in, the way a message to the agent is written. While it is open,
// anything clicked on screen drops into the sentence at the caret as a numbered reference, and
// a drag drops in a screenshot. Typing @ finds things by name, for when the hands are on the
// keyboard. Enter copies the message and closes. Esc closes and keeps what was written.

import { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { Overlay } from "../../ui/Overlay";
import { named } from "./names";
import { describe, mark, outline, pointing } from "./pointing";
import type { Thing } from "./pointing";
import { lab, SHORTCUT } from "./stage";
import type { Lab } from "./stage";
import { selector } from "../../ui/web";

const WIDTH = 560;
const WHERE = 'Google Chrome "Invoices"';
/** What a reference is called inside the sentence: the words on it, without its kind. */
const short = (thing: Thing) => {
  const quoted = thing.headline.match(/"(.*)"/)?.[1] ?? thing.headline;
  return quoted.length > 24 ? `${quoted.slice(0, 23)}…` : quoted;
};

interface Found {
  element: Element;
  name: string;
}

function Composer({ stage }: { stage: Lab }) {
  const [open, setOpen] = useState(false);
  const [empty, setEmpty] = useState(true);
  const [said, setSaid] = useState("");
  const [home, setHome] = useState({ left: 0, top: 0 });
  const [menu, setMenu] = useState<{ found: Found[]; index: number } | null>(null);
  const editor = useRef<HTMLDivElement>(null);
  const things = useRef(new Map<number, Thing>());
  const caret = useRef<Range | null>(null);
  const point = useRef<{ set(on: boolean): void } | null>(null);
  const live = useRef({ open, menu });
  live.current = { open, menu };

  const chips = () => [...(editor.current as HTMLElement).querySelectorAll<HTMLElement>(".ref")];
  // The things in the order the sentence mentions them; that order is their numbers.
  const mentioned = () => chips().map((chip) => things.current.get(Number(chip.dataset.id)) as Thing);
  const sync = () => {
    const order = [...new Set(chips().map((chip) => Number(chip.dataset.id)))];
    chips().forEach((chip) => ((chip.firstElementChild as HTMLElement).textContent = String(order.indexOf(Number(chip.dataset.id)) + 1)));
    for (const id of [...things.current.keys()]) if (!order.includes(id)) things.current.delete(id);
    mark(stage, mentioned());
    setEmpty((editor.current as HTMLElement).textContent === "");
  };
  const keep = () => {
    const selection = window.getSelection();
    if (selection?.rangeCount && editor.current?.contains(selection.anchorNode)) caret.current = selection.getRangeAt(0).cloneRange();
  };
  const before = (range: Range) => {
    const all = document.createRange();
    all.selectNodeContents(editor.current as HTMLElement);
    all.setEnd(range.startContainer, range.startOffset);
    return all.toString();
  };

  const insert = (thing: Thing) => {
    const box = editor.current as HTMLElement;
    // The same element again is the same thing: it keeps its number.
    const same = thing.kind === "element" ? [...things.current.values()].find((t) => t.target === thing.target) : undefined;
    const kept = same ?? thing;
    things.current.set(kept.id, kept);
    let range = caret.current;
    if (!range || !box.contains(range.startContainer)) {
      range = document.createRange();
      range.selectNodeContents(box);
      range.collapse(false);
    }
    range.deleteContents();
    const chip = document.createElement("span");
    chip.className = "ref";
    chip.contentEditable = "false";
    chip.dataset.id = String(kept.id);
    chip.append(document.createElement("b"), short(kept));
    const lead = before(range);
    const space = document.createTextNode(" ");
    const pieces = document.createDocumentFragment();
    if (lead !== "" && !/\s$/.test(lead)) pieces.append(" ");
    pieces.append(chip, space);
    range.insertNode(pieces);
    range.setStart(space, 1);
    range.collapse(true);
    const selection = window.getSelection() as Selection;
    selection.removeAllRanges();
    selection.addRange(range);
    caret.current = range.cloneRange();
    box.focus();
    sync();
  };

  // "@new" just before the caret: the things on the page whose names have that in them.
  const find = () => {
    const selection = window.getSelection();
    const node = selection?.anchorNode;
    if (!node || node.nodeType !== Node.TEXT_NODE || !editor.current?.contains(node)) return setMenu(null);
    const match = (node.textContent ?? "").slice(0, selection.anchorOffset).match(/(?:^|\s)@([^@\s]{0,20})$/);
    if (!match) return setMenu(null);
    const query = match[1].toLowerCase();
    const found: Found[] = [];
    for (const frame of stage.frames) {
      for (const element of (frame.contentDocument as Document).querySelectorAll("button, a, input, select, textarea, h1, h2, h3, table, [id]")) {
        const rect = stage.rectOf(element);
        const name = named(element);
        if (rect.width === 0 || !`${name} ${element.id}`.toLowerCase().includes(query)) continue;
        found.push({ element, name });
        if (found.length === 6) break;
      }
    }
    setMenu(found.length ? { found, index: 0 } : null);
  };
  const choose = (found: Found) => {
    const selection = window.getSelection() as Selection;
    const node = selection.anchorNode as Text;
    const start = (node.textContent ?? "").lastIndexOf("@", selection.anchorOffset);
    const range = document.createRange();
    range.setStart(node, start);
    range.setEnd(node, selection.anchorOffset);
    caret.current = range;
    setMenu(null);
    insert({ id: Date.now(), kind: "element", headline: found.name, selector: selector(found.element), target: found.element });
    outline(stage, null);
  };
  useEffect(() => {
    // The one the arrow keys are on is outlined on the page.
    if (menu) outline(stage, { id: 0, kind: "element", headline: "", selector: "", target: menu.found[menu.index].element });
  }, [menu, stage]);

  const text = () => {
    const order = [...new Set(chips().map((chip) => Number(chip.dataset.id)))];
    const read = (node: Node): string => {
      if (node.nodeType === Node.TEXT_NODE) return node.textContent ?? "";
      const element = node as HTMLElement;
      if (element.classList.contains("ref")) return `[${order.indexOf(Number(element.dataset.id)) + 1}]`;
      if (element.tagName === "BR") return "\n";
      return (element.tagName === "DIV" ? "\n" : "") + [...element.childNodes].map(read).join("");
    };
    const sentence = [...(editor.current as HTMLElement).childNodes].map(read).join("").replace(/ /g, " ").replace(/ +/g, " ").replace(/ ([.,;:!?])/g, "$1").trim();
    const list = order.map((id, index) => `[${index + 1}] ${describe(things.current.get(id) as Thing, index + 1)}`);
    if (list.length === 0) return sentence;
    return [sentence, "", ...list, `[Clipframes: ${list.length === 1 ? "1 thing" : `${list.length} things`} in ${WHERE}]`].join("\n").trim();
  };

  const show = () => {
    const display = stage.displayAt(stage.pointer.x, stage.pointer.y);
    setHome((h) => (h.left || h.top ? h : { left: display.x + (display.width - WIDTH) / 2, top: display.y + display.height - 150 }));
    setSaid("");
    setOpen(true);
    point.current?.set(true);
    mark(stage, mentioned());
    window.setTimeout(() => {
      const box = editor.current as HTMLElement;
      box.focus();
      const range = document.createRange();
      range.selectNodeContents(box);
      range.collapse(false);
      window.getSelection()?.removeAllRanges();
      window.getSelection()?.addRange(range);
      caret.current = range.cloneRange();
    });
  };
  const hide = () => {
    setOpen(false);
    setMenu(null);
    point.current?.set(false);
    mark(stage, []);
  };
  const copy = () => {
    const message = text();
    if (message === "") return hide();
    stage.clipboard.set(message);
    (editor.current as HTMLElement).replaceChildren();
    things.current.clear();
    caret.current = null;
    setEmpty(true);
    hide();
    setSaid("Copied. Paste it into your agent.");
    window.setTimeout(() => setSaid(""), 2600);
  };

  useEffect(() => {
    point.current = pointing(stage, insert);
    stage.onShortcut(() => (live.current.open ? hide() : show()));
    stage.onKey("keydown", (event) => {
      if (event.key === "Escape" && live.current.open && !live.current.menu) hide();
    });
    document.addEventListener("selectionchange", keep);
    stage.ready();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  useEffect(() => stage.hint(open ? "" : `${SHORTCUT} to tell your agent what to change`), [open, stage]);

  const grip = (event: React.PointerEvent) => {
    event.preventDefault();
    const start = { x: event.clientX - home.left, y: event.clientY - home.top };
    const move = (e: PointerEvent) => setHome({ left: Math.max(0, Math.min(e.clientX - start.x, stage.glass.clientWidth - WIDTH)), top: Math.max(0, Math.min(e.clientY - start.y, stage.glass.clientHeight - 110)) });
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  return (
    <div className="clipframes-web">
      <Overlay />
      <div className="window composer" hidden={!open} style={{ ...home, width: WIDTH }}>
        {menu && (
          <div className="found">
            {menu.found.map((found, index) => (
              <button key={index} className={index === menu.index ? "on" : ""} onPointerEnter={() => setMenu({ ...menu, index })} onPointerDown={(e) => (e.preventDefault(), choose(found))}>
                {found.name}
              </button>
            ))}
          </div>
        )}
        <div
          ref={editor}
          className={empty ? "say empty" : "say"}
          contentEditable
          suppressContentEditableWarning
          spellCheck={false}
          role="textbox"
          aria-label="What should change"
          data-placeholder="What should change? Click things on screen as you write."
          onBeforeInput={(e) => {
            // A full stop or a comma typed straight after a reference sits against it: the
            // space that was put after the reference gives way.
            const typed = (e.nativeEvent as InputEvent).data ?? "";
            const selection = window.getSelection();
            const node = selection?.anchorNode;
            if (!/^[.,;:!?)]/.test(typed) || !node || node.nodeType !== Node.TEXT_NODE || selection?.anchorOffset !== 1) return;
            if (node.textContent?.[0] === " " && (node.previousSibling as Element | null)?.classList?.contains("ref")) (node as Text).deleteData(0, 1);
          }}
          onInput={() => {
            sync();
            find();
          }}
          onPointerOver={(e) => {
            const chip = (e.target as Element).closest<HTMLElement>(".ref");
            outline(stage, chip ? (things.current.get(Number(chip.dataset.id)) ?? null) : null);
          }}
          onPointerLeave={() => outline(stage, null)}
          onKeyDown={(e) => {
            if (menu) {
              if (e.key === "ArrowDown" || e.key === "ArrowUp") {
                e.preventDefault();
                return setMenu({ ...menu, index: (menu.index + (e.key === "ArrowDown" ? 1 : menu.found.length - 1)) % menu.found.length });
              }
              if (e.key === "Enter" || e.key === "Tab") return e.preventDefault(), choose(menu.found[menu.index]);
              if (e.key === "Escape") return e.stopPropagation(), setMenu(null), outline(stage, null);
            }
            if (e.key !== "Enter" || e.shiftKey || e.nativeEvent.isComposing) return;
            e.preventDefault();
            copy();
          }}
        />
        <div className="foot">
          <span className="grip" title="Drag to move" onPointerDown={grip}>
            <i /><i /><i /><i /><i /><i />
          </span>
          <span className="how">Click to point. Drag for a screenshot. @ finds by name.</span>
          <button className="pill" onClick={copy}>
            Copy <kbd>⏎</kbd>
          </button>
        </div>
      </div>
      {said && (
        <div className="window toast" style={{ left: home.left + WIDTH / 2, top: home.top + 40, bottom: "auto" }}>
          <span>{said}</span>
        </div>
      )}
    </div>
  );
}

const stage = await lab();
const root = document.createElement("div");
stage.glass.appendChild(root);
createRoot(root).render(<Composer stage={stage} />);
