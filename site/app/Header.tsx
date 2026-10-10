"use client";

import { useEffect, useState } from "react";
import { track } from "@/lib/analytics";
import { DOWNLOAD_PATH, REPO_URL } from "@/lib/site";
import { formatStars } from "@/lib/github";
import { CopyLink, DownloadIcon } from "./DownloadButton";
import s from "./home.module.css";

/// The quiet top bar. A hairline appears under it once the page scrolls. `stars` is the count the
/// server had when it built the page; the bar refreshes it from /stars.json after loading.
export default function Header({ stars: initial, refresh = true }: { stars: number | null; refresh?: boolean }) {
  const [scrolled, setScrolled] = useState(false);
  const [stars, setStars] = useState(initial);
  useEffect(() => {
    if (!refresh) return;
    let live = true;
    fetch("/stars.json")
      .then((r) => (r.ok ? r.json() : null))
      .then((d) => { if (live && typeof d?.stars === "number") setStars(d.stars); })
      .catch(() => {});
    return () => { live = false; };
  }, [refresh]);
  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 8);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <header className={`${s.header} ${scrolled ? s.scrolled : ""}`}>
      <div className={s.headerIn}>
        <a className={s.brand} href="/">Clipframes</a>
        <a className={s.gh} href={REPO_URL} aria-label={stars ? `GitHub, ${stars} ${stars === 1 ? "star" : "stars"}` : "GitHub"}>
          GitHub
          {/* No badge at zero: an empty counter says the wrong thing. */}
          {!!stars && (
            <span className={s.stars}>
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 2.6l2.9 5.9 6.5.9-4.7 4.6 1.1 6.5L12 17.4l-5.8 3.1 1.1-6.5L2.6 9.4l6.5-.9z" /></svg>
              {formatStars(stars)}
            </span>
          )}
        </a>
        <span className={s.desk}><a className={s.pill} href={DOWNLOAD_PATH} onClick={() => track("download_clicked", { os: "auto", place: "header" })}><DownloadIcon />Download</a></span>
        <span className={s.phone}><CopyLink place="header" /></span>
      </div>
    </header>
  );
}
