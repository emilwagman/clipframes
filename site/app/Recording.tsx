"use client";

import { useEffect, useRef, useState } from "react";
import { track } from "@/lib/analytics";
import { RECORDINGS } from "@/lib/media";
import s from "./home.module.css";

/// A real recording of the app. Nothing of it is fetched until it comes near the screen, it
/// plays only while it is on screen, and its box has the recording's shape from the start.
export default function Recording({ name, label }: { name: keyof typeof RECORDINGS; label: string }) {
  const media = RECORDINGS[name];
  const video = useRef<HTMLVideoElement>(null);
  const [near, setNear] = useState(false);
  const [playing, setPlaying] = useState(false);
  /// The visitor pressed Pause: it stays paused when it comes back on screen.
  const held = useRef(false);
  const counted = useRef(false);
  const onScreen = useRef(false);

  useEffect(() => {
    const node = video.current;
    if (!node || typeof IntersectionObserver === "undefined") return;
    const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const close = new IntersectionObserver(([entry]) => entry.isIntersecting && setNear(true), { rootMargin: "700px 0px" });
    const shown = new IntersectionObserver(([entry]) => {
      onScreen.current = entry.isIntersecting && !still;
      if (onScreen.current && !held.current && node.currentSrc) void node.play().catch(() => {});
      else node.pause();
    }, { threshold: 0.5 });
    close.observe(node);
    shown.observe(node);
    return () => (close.disconnect(), shown.disconnect());
  }, []);

  // It may be on screen before there is anything to play: a jump straight to it.
  useEffect(() => {
    if (near && onScreen.current && !held.current) void video.current?.play().catch(() => {});
  }, [near]);

  const toggle = () => {
    const node = video.current;
    if (!node) return;
    held.current = !node.paused;
    if (node.paused) void node.play().catch(() => {});
    else node.pause();
  };

  return (
    <figure className={s.recording} data-recording={name}>
      <video
        ref={video}
        src={near ? media.src : undefined}
        poster={near ? media.poster : undefined}
        width={media.width}
        height={media.height}
        style={{ aspectRatio: `${media.width} / ${media.height}` }}
        preload="none"
        muted
        loop
        playsInline
        aria-label={label}
        onClick={toggle}
        onPlay={() => {
          setPlaying(true);
          if (!counted.current) track("recording_played", { place: name });
          counted.current = true;
        }}
        onPause={() => setPlaying(false)}
      />
      <figcaption>
        <span>{label}</span>
        <button onClick={toggle}>{playing ? "Pause" : "Play"}</button>
      </figcaption>
    </figure>
  );
}
