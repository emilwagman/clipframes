"use client";

import { useEffect, useMemo, useState } from "react";
import { History } from "@desktop/src/History";
import type { HistoryApi } from "@desktop/src/History";
import one from "@desktop/lab/thumbs/1.png";
import two from "@desktop/lab/thumbs/2.png";
import three from "@desktop/lab/thumbs/3.png";
import four from "@desktop/lab/thumbs/4.png";
import { copyToClipboard, useSite } from "./engine";
import type { Capture } from "./engine";
import s from "./history.module.css";

/// Earlier captures to start the list with: the app's own examples (desktop/lab/lab.tsx).
const EARLIER: Capture[] = [
  { id: "a", title: 'Button "New invoice" and 2 more', when: "2026-10-09 11:42", count: 3, images: [one.src, two.src, three.src],
    text: '[Clipframes: 3 things in Google Chrome "Invoices"]\n1. Button "New invoice" (#new-invoice .btn.btn-primary): make this green\n2. Screenshot (2.png): the table is cramped\n3. Screen clip, 6 s, 24 frames (3/)' },
  { id: "b", title: "Screen clip, 6 s", when: "2026-10-09 10:15", count: 1, images: [four.src], text: '[Screen clip, 6 s, 24 frames (1/) in Google Chrome "Invoices"]' },
  { id: "c", title: 'Text "$3,120" and 1 more', when: "2026-10-08 16:03", count: 2, images: [three.src, two.src],
    text: '[Clipframes: 2 things in Google Chrome "Invoices"]\n1. Text "$3,120" (#overdue-total): too alarming, use the normal text colour\n2. Group "Outstanding $12,940" (#outstanding-total .stat)' },
  { id: "d", title: "Screenshot", when: "2026-10-08 09:27", count: 1, images: [two.src], text: '[Screenshot (1.png) in Google Chrome "Invoices": the table is cramped]' },
];

/// The app's History window, listing what the visitor picked on this page above a few earlier
/// captures. Copy puts a capture's text on the clipboard again, as it does in the app.
export default function HistoryDemo() {
  const captures = useSite((site) => site.captures);
  const [deleted, setDeleted] = useState<string[]>([]);

  const api = useMemo<HistoryApi>(() => {
    const entries = [...captures, ...EARLIER].filter((entry) => !deleted.includes(entry.id));
    return {
      list: async () => ({ entries, total: entries.length, next: entries.length }),
      copy: async (id) => {
        const text = entries.find((entry) => entry.id === id)?.text;
        if (!text) return;
        copyToClipboard(text);
        useSite.setState({ latest: text });
      },
      // There is no folder to open on a website.
      reveal: async () => {},
      remove: async (id) => setDeleted((before) => [...before, id]),
      src: (path) => path,
    };
  }, [captures, deleted]);

  // History is a window of its own in the app and marks the whole page as one. Here it is a
  // panel in a page, and that mark would tell the demos that every click is on a window.
  useEffect(() => document.documentElement.classList.remove("window"));

  return (
    <div className={`cf ${s.panel}`}>
      <History api={api} />
    </div>
  );
}
