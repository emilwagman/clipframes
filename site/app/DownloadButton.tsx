"use client";

import { useState } from "react";
import { track } from "@/lib/analytics";
import { DOWNLOAD_MAC, DOWNLOAD_PATH, DOWNLOAD_WINDOWS, SITE_URL } from "@/lib/site";
import s from "./home.module.css";

/// Where on the page a download button is.
export type Place = "header" | "hero" | "below" | "end";

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

/// One button, for the first screen: the download for the system the visitor is on (app/download).
export function DownloadOne({ place }: { place: Place }) {
  return <a className={`${s.pill} ${s.big}`} href={DOWNLOAD_PATH} onClick={() => track("download_clicked", { os: "auto", place })}><DownloadIcon />Download Clipframes</a>;
}

/// The two downloads as words in a sentence, for the quiet line under the first screen.
export function DownloadLinks({ place }: { place: Place }) {
  return (
    <>
      <a href={DOWNLOAD_MAC} onClick={() => track("download_clicked", { os: "mac", place })}>Download for Mac</a>
      {" or "}
      <a href={DOWNLOAD_WINDOWS} onClick={() => track("download_clicked", { os: "windows", place })}>Download for Windows</a>
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
