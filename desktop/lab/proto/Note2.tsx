// The comment box as one line: a third of the height of ui/Note.tsx, so it covers less of
// the page. It grows as lines are added. Enter saves, Esc closes, the cross takes the pick back.

import { useEffect, useRef, useState } from "react";
import { Icon } from "../../ui/icons";
import { usePlatform, useRound } from "../../ui/store";

export function Note2() {
  const platform = usePlatform();
  const round = useRound();
  const index = round.noting;
  const pick = index === null ? undefined : round.picks[index];
  const [text, setText] = useState("");
  const input = useRef<HTMLTextAreaElement>(null);
  const open = useRef<{ index: number; text: string } | null>(null);

  useEffect(() => {
    const before = open.current;
    if (before && before.index !== index && before.text.trim() !== "") void platform.setNote(before.index, before.text);
    const start = pick?.note ?? "";
    open.current = index === null ? null : { index, text: start };
    setText(start);
    input.current?.focus();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [index]);
  useEffect(() => {
    const node = input.current;
    if (!node) return;
    node.style.height = "20px";
    node.style.height = `${Math.min(60, node.scrollHeight)}px`;
  }, [text, index]);

  if (index === null || !pick) return null;
  const save = async () => {
    if (text.trim() !== "") await platform.setNote(index, text);
    open.current = null;
    await platform.closeNote();
  };

  return (
    <div className="note slim">
      <b className="num">{index + 1}</b>
      <textarea
        ref={input}
        rows={1}
        value={text}
        placeholder="What should change?"
        spellCheck={false}
        onChange={(e) => {
          setText(e.target.value);
          open.current = { index, text: e.target.value };
        }}
        onKeyDown={(e) => {
          if (e.key !== "Enter" || e.shiftKey || e.nativeEvent.isComposing) return;
          e.preventDefault();
          void save();
        }}
      />
      <button
        className="round small"
        aria-label="Remove this pick"
        title="Remove this pick"
        onClick={() => {
          open.current = null;
          void platform.removePick(index);
        }}
      >
        <Icon name="trash" />
      </button>
    </div>
  );
}
