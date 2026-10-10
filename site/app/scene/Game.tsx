"use client";

// Hopper: a made-up game, drawn on one canvas as a game draws itself. There are no buttons or
// labels in it for anything to read, which is why the example shows a place in it with an
// area. As it starts, the score is drawn over the health bar: the fault the example points at.

import { useEffect, useRef } from "react";

const SKY = "#cfe8ff";

function draw(g: CanvasRenderingContext2D, w: number, h: number, t: number, mended: boolean): void {
  const ground = h * 0.78;
  g.fillStyle = SKY;
  g.fillRect(0, 0, w, h);
  // Far hills, clouds that drift, the ground.
  g.fillStyle = "#a9d9a2";
  for (const [x, r] of [[0.18, 0.3], [0.55, 0.38], [0.9, 0.28]]) { g.beginPath(); g.ellipse(w * x, ground, w * r, h * 0.26, 0, Math.PI, 0); g.fill(); }
  g.fillStyle = "#ffffff";
  for (const [x, y, s] of [[0.2, 0.3, 1], [0.62, 0.2, 1.3], [0.92, 0.36, 0.8]]) {
    const cx = ((w * x + t * 9 * s) % (w + 160)) - 80;
    g.beginPath(); g.roundRect(cx, h * y, 74 * s, 20 * s, 10 * s); g.fill();
  }
  g.fillStyle = "#6fae4f";
  g.fillRect(0, ground, w, 14);
  g.fillStyle = "#8a6a45";
  g.fillRect(0, ground + 14, w, h - ground - 14);
  // A ledge, three coins, and Hopper, who hops.
  g.fillStyle = "#8a6a45";
  g.beginPath(); g.roundRect(w * 0.56, ground - 96, 150, 20, 6); g.fill();
  g.fillStyle = "#6fae4f";
  g.beginPath(); g.roundRect(w * 0.56, ground - 100, 150, 10, 5); g.fill();
  g.fillStyle = "#f2b705";
  for (let i = 0; i < 3; i++) { g.beginPath(); g.arc(w * 0.56 + 30 + i * 44, ground - 126 + Math.sin(t * 3 + i) * 3, 9, 0, Math.PI * 2); g.fill(); }
  const hop = Math.abs(Math.sin(t * 2.6)) * 74;
  const x = w * 0.3 + Math.sin(t * 0.9) * w * 0.07;
  const squash = 1 - Math.max(0, 1 - hop / 16) * 0.16;
  g.fillStyle = "rgba(23, 24, 28, 0.16)";
  g.beginPath(); g.ellipse(x + 21, ground + 4, 22 - hop * 0.1, 5, 0, 0, Math.PI * 2); g.fill();
  g.fillStyle = "#3b3fd8";
  g.beginPath(); g.roundRect(x, ground - 42 * squash - hop, 42, 42 * squash, 10); g.fill();
  g.fillStyle = "#ffffff";
  g.fillRect(x + 24, ground - 30 * squash - hop, 6, 8);
  g.fillRect(x + 33, ground - 30 * squash - hop, 6, 8);
  // The heads-up display: health, and the score, which is drawn on top of it until it is mended.
  g.fillStyle = "#17181c";
  g.beginPath(); g.roundRect(16, 16, 152, 20, 6); g.fill();
  g.fillStyle = "#e5484d";
  g.beginPath(); g.roundRect(19, 19, 104, 14, 4); g.fill();
  g.fillStyle = "#17181c";
  g.font = '700 15px ui-monospace, "SF Mono", "Cascadia Mono", Menlo, Consolas, monospace';
  g.textBaseline = "top";
  g.fillText("SCORE 01280", mended ? 16 : 70, mended ? 44 : 20);
}

/// `changed` is the game after the agent has done what the example asked: the score is under the health bar.
export default function Game({ changed = false }: { changed?: boolean }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const node = canvas.current;
    const g = node?.getContext("2d");
    if (!node || !g) return;
    const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
    let frame = 0;
    const paint = (now: number) => {
      const ratio = Math.min(devicePixelRatio || 1, 2);
      const { clientWidth: w, clientHeight: h } = node;
      if (node.width !== Math.round(w * ratio) || node.height !== Math.round(h * ratio)) {
        node.width = Math.round(w * ratio);
        node.height = Math.round(h * ratio);
      }
      g.setTransform(ratio, 0, 0, ratio, 0, 0);
      draw(g, w, h, still ? 0.42 : now / 1000, changed);
      if (!still) frame = requestAnimationFrame(paint);
    };
    frame = requestAnimationFrame(paint);
    return () => cancelAnimationFrame(frame);
  }, [changed]);
  return <canvas ref={canvas} id="view" aria-label="A small platform game: a blue square hops along the grass, with a health bar and a score in the top left corner." style={{ position: "absolute", inset: 0, width: "100%", height: "100%", display: "block", background: SKY }} />;
}
