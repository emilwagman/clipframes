// The comment box: what should change about the thing just picked. Optional: clicking the next
// thing, or Enter on an empty box, moves on.

import { useEffect, useRef, useState } from "react";
import { Icon } from "./icons";
import { usePlatform, useRound, useStore } from "./store";

/**
 * The comment being written for the pick that was just made, wherever it is typed: in the box
 * at the pick, or in the bar. Only one window writes it, so the others pass `active` false.
 */
export function useComment<Field extends HTMLTextAreaElement | HTMLInputElement>(active: boolean) {
  const platform = usePlatform();
  const round = useRound();
  const index = active ? round.noting : null;
  const pick = index === null ? undefined : round.picks[index];
  const [text, setText] = useState("");
  const input = useRef<Field>(null);
  // What was being written, so it can be kept when the next pick takes the box over.
  const open = useRef<{ index: number; text: string } | null>(null);

  useEffect(() => {
    const before = open.current;
    if (before && before.index !== index && before.text.trim() !== "") void platform.setNote(before.index, before.text);
    const start = pick?.note ?? "";
    open.current = index === null ? null : { index, text: start };
    setText(start);
    input.current?.focus();
    // Only a change of pick resets the box, not every keystroke echoing back.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [index]);

  const write = (value: string) => {
    if (index === null) return;
    setText(value);
    open.current = { index, text: value };
    // Sent as it is typed: the round can be closed in ways this box never hears about (the
    // shortcut, the bar, quitting), and what was written must not go with it.
    void platform.setNote(index, value);
  };
  const save = async () => {
    if (index === null) return;
    if (text.trim() !== "") await platform.setNote(index, text);
    open.current = null;
    await platform.closeNote();
  };
  const remove = async () => {
    if (index === null) return;
    open.current = null;
    await platform.removePick(index);
  };
  /** `byKey`: Backspace in an empty comment takes the pick back, where there is no button for it. */
  const onKeyDown = (byKey: boolean) => (e: React.KeyboardEvent<Field>) => {
    // A key held down to rub out the words must not go on to rub out the pick.
    if (byKey && e.key === "Backspace" && text === "" && !e.repeat) {
      e.preventDefault();
      return void remove();
    }
    // Enter saves; Shift+Enter is a new line. Enter that confirms an input method is left alone.
    if (e.key !== "Enter" || e.shiftKey || e.nativeEvent.isComposing) return;
    e.preventDefault();
    void save();
  };
  return { index, pick, text, input, write, save, remove, onKeyDown };
}

export function Note() {
  const platform = usePlatform();
  const style = useRound().noteStyle;
  const faint = useStore((s) => s.faint);
  // The one-line box: a third as tall, so it covers less of what is picked next. As "ghost"
  // it has nothing to click: the pointer goes through it to what it covers.
  const slim = style === "line" || style === "ghost";
  const ghost = style === "ghost";
  const { index, pick, text, input, write, save, remove, onKeyDown } = useComment<HTMLTextAreaElement>(style !== "dock");
  const card = useRef<HTMLDivElement>(null);

  // The one-line box is as tall as what is written in it, up to three lines, and its window
  // with it: 8px of room for the shadow on each side.
  useEffect(() => {
    const node = input.current;
    if (!slim || !node || !card.current) return;
    node.style.height = "26px";
    node.style.height = `${Math.min(66, node.scrollHeight)}px`;
    void platform.resizeNote?.(card.current.offsetHeight + 16);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [text, index, slim]);

  if (index === null || !pick) return null;

  if (slim) {
    // The pick's number instead of its name, and no Save: Enter saves, and so does picking the next thing.
    return (
      <div className={ghost ? (faint ? "note slim ghost faint" : "note slim ghost") : "note slim"} ref={card}>
        <b className="num" title={pick.headline}>
          {index + 1}
        </b>
        <textarea ref={input} id="comment" rows={1} value={text} placeholder="What should change?" spellCheck={false} autoComplete="off" onChange={(e) => write(e.target.value)} onKeyDown={onKeyDown(ghost)} />
        {!ghost && (
          <button className="round small" aria-label="Remove this pick" title="Remove this pick" onClick={() => void remove()}>
            <Icon name="trash" />
          </button>
        )}
      </div>
    );
  }

  return (
    <div className="note">
      <div className="what">{pick.headline}</div>
      <textarea
        ref={input}
        id="comment"
        rows={2}
        value={text}
        placeholder="What should change?"
        spellCheck={false}
        autoComplete="off"
        onChange={(e) => write(e.target.value)}
        onKeyDown={onKeyDown(false)}
      />
      <div className="foot">
        <button className="quiet" onClick={() => void remove()}>
          Remove
        </button>
        <button className="pill" onClick={() => void save()}>
          Save
        </button>
      </div>
    </div>
  );
}
