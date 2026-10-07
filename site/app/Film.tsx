"use client";

import { useEffect, useRef } from "react";
import s from "./home.module.css";

/// A real recording of the app: silent, looping, playing only while on screen.
/// With reduced motion it stays on its poster frame. Without a source yet, it holds the space.
export default function Film({ src, poster, label }: { src?: string; poster?: string; label: string }) {
  const ref = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    const video = ref.current;
    if (!video) return;
    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const io = new IntersectionObserver(([e]) => {
      if (e.isIntersecting && !still) video.play().catch(() => {});
      else video.pause();
    }, { threshold: 0.35 });
    io.observe(video);
    return () => io.disconnect();
  }, []);

  return (
    <figure className={s.figure}>
      <div className={s.film}>
        {src
          ? <video ref={ref} src={src} poster={poster} muted loop playsInline preload="metadata" aria-label={label} />
          : <div className={s.soon}>Recording coming</div>}
      </div>
    </figure>
  );
}
