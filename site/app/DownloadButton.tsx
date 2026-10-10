"use client";

import { useState } from "react";
import { track } from "@/lib/analytics";
import { DOWNLOAD_MAC, DOWNLOAD_WINDOWS, SITE_URL } from "@/lib/site";
import s from "./home.module.css";

/// Where on the page a download button is.
export type Place = "header" | "hero" | "end";

export function DownloadIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M12 4v12M6.5 10.5L12 16l5.5-5.5M5 20h14" />
    </svg>
  );
}

/// The two download buttons, one per system. `place` says which pair on the page was used.
export function DownloadButtons({ place }: { place: Place }) {
  return (
    <>
      <a className={`${s.pill} ${s.big}`} href={DOWNLOAD_MAC} onClick={() => track("download_clicked", { os: "mac", place })}><DownloadIcon />Download for Mac</a>
      <a className={`${s.pill} ${s.big}`} href={DOWNLOAD_WINDOWS} onClick={() => track("download_clicked", { os: "windows", place })}><DownloadIcon />Download for Windows</a>
    </>
  );
}

/// What a phone gets in place of a download it cannot use: the site's address on the
/// clipboard, to open on a computer. Nothing is sent anywhere.
export function CopyLink({ place, big = false }: { place: Place; big?: boolean }) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    track("link_copied", { place });
    const done = () => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2400);
    };
    try {
      navigator.clipboard.writeText(SITE_URL).then(done, () => {});
    } catch {
      /* no clipboard here; the address is written beside the button */
    }
  };
  return (
    <button className={`${s.pill} ${big ? s.big : ""}`} data-copy-link={place} onClick={copy} aria-live="polite">
      {copied ? "Link copied" : big ? "Copy the link to this page" : "Copy link"}
    </button>
  );
}
