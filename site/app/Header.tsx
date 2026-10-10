"use client";

import { useEffect, useState } from "react";
import { track } from "@/lib/analytics";
import { DOWNLOAD_PATH, PAGES, REPO_URL } from "@/lib/site";
import { formatStars } from "@/lib/github";
import { CopyLink, DownloadIcon } from "./DownloadButton";
import s from "./home.module.css";

function Words({ here }: { here?: string }) {
  return (
    <>
      {PAGES.map((page) => page.href
        ? <a key={page.name} href={page.href} aria-current={page.href === here ? "page" : undefined}>{page.name}</a>
        : <span key={page.name} title="Not built yet">{page.name}</span>)}
    </>
  );
}

/// The quiet top bar: the name, where the answers are, and the download. A hairline appears
/// under it once the page scrolls. `stars` is the count the server had when it built the page;
/// the bar refreshes it from /stars.json after loading. `home` is the home page, whose first
/// screen has the download in it: the bar's own shows once that has scrolled away.
export default function Header({ stars: initial, refresh = true, here, home = false }: { stars: number | null; refresh?: boolean; here?: string; home?: boolean }) {
  const [scrolled, setScrolled] = useState(0);
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
    const onScroll = () => setScrolled(window.scrollY > 420 ? 2 : window.scrollY > 8 ? 1 : 0);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <header className={`${s.header} ${scrolled ? s.scrolled : ""} ${scrolled === 2 ? s.scrolledFar : ""}`}>
      <div className={s.headerIn}>
        <a className={s.brand} href="/">Clipframes</a>
        <nav className={s.nav} aria-label="Pages"><Words here={here} /></nav>
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
        <details className={s.menu}>
          <summary>Menu</summary>
          <nav aria-label="Pages"><Words here={here} /><a href={REPO_URL}>GitHub</a></nav>
        </details>
        <span className={`${s.desk} ${home ? s.late : ""}`}><a className={s.pill} href={DOWNLOAD_PATH} onClick={() => track("download_clicked", { os: "auto", place: "header" })}><DownloadIcon />Download</a></span>
        <span className={s.phone}><CopyLink place="header" /></span>
      </div>
    </header>
  );
}
