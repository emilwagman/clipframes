// The comment box: what should change about the thing just picked. Optional: clicking the next
// thing, or Enter on an empty box, moves on.

import { useEffect, useRef, useState } from "react";
import { usePlatform, useRound } from "./store";

export function Note() {
  const platform = usePlatform();
  const round = useRound();
  const index = round.noting;
  const pick = index === null ? undefined : round.picks[index];
  const [text, setText] = useState("");
  const input = useRef<HTMLTextAreaElement>(null);
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

  if (index === null || !pick) return null;

  const write = (value: string) => {
    setText(value);
    open.current = { index, text: value };
  };
  const save = async () => {
    if (text.trim() !== "") await platform.setNote(index, text);
    open.current = null;
    await platform.closeNote();
  };
  const remove = async () => {
    open.current = null;
    await platform.removePick(index);
  };

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
        onKeyDown={(e) => {
          // Enter saves; Shift+Enter is a new line. Enter that confirms an input method is left alone.
          if (e.key !== "Enter" || e.shiftKey || e.nativeEvent.isComposing) return;
          e.preventDefault();
          void save();
        }}
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
