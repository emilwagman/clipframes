"use client";

import { useRef, useState } from "react";
import { track } from "@/lib/analytics";
import { RECORDINGS } from "@/lib/media";
import s from "./scene.module.css";

/// The proof that the desktop app in the scene is not a fiction: a recording of the real app
/// picking in the real Focus on Windows and pasting into Claude Code. It is a few words with a
/// play mark until it is asked for; nothing of the recording is fetched before that.
export default function Proof() {
  const dialog = useRef<HTMLDialogElement>(null);
  const [open, setOpen] = useState(false);
  const media = RECORDINGS.native;
  const show = () => {
    setOpen(true);
    track("recording_played", { place: "native" });
    dialog.current?.showModal();
  };
  return (
    <>
      <button className={s.proof} data-proof="" onClick={show}>
        <i aria-hidden="true" />The real app, recorded on Windows
      </button>
      <dialog ref={dialog} className={s.film} onClose={() => setOpen(false)} onClick={(event) => event.target === dialog.current && dialog.current?.close()}>
        {open && <video src={media.src} poster={media.poster} width={media.width} height={media.height} controls autoPlay muted playsInline aria-label="A recording of Clipframes on Windows, 26 seconds long. Two buttons are picked in Focus, a desktop app, and the text is pasted into Claude Code." />}
        <form method="dialog"><button>Close</button></form>
      </dialog>
    </>
  );
}
