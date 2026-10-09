"use client";

import { useEffect, useState } from "react";
import { DOWNLOAD_PATH, REPO_URL } from "@/lib/site";
import { DownloadIcon } from "./DownloadButton";
import s from "./home.module.css";

/// The quiet top bar. A hairline appears under it once the page scrolls.
export default function Header() {
  const [scrolled, setScrolled] = useState(false);
  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 8);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <header className={`${s.header} ${scrolled ? s.scrolled : ""}`}>
      <div className={s.headerIn}>
        <a className={s.brand} href="/">
          <img src="/wordmark.png" alt="Clipframes" width={127} height={40} className={s.wordmark} />
        </a>
        <a className={s.gh} href={REPO_URL}>GitHub</a>
        <a className={s.pill} href={DOWNLOAD_PATH}><DownloadIcon />Download</a>
      </div>
    </header>
  );
}
