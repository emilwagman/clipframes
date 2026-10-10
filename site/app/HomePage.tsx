// The home page itself. It is apart from page.tsx so the preview build can draw it too
// (scripts/build-preview.mjs).
//
// This is the pilot of the page's first screen (design/rulebook.md, 2026-10-10): the headline,
// one line, one download, and the scene, which shows the rest. What comes under it is not
// built yet.

import Footer from "./Footer";
import Header from "./Header";
import { DownloadOne } from "./DownloadButton";
import Live from "./demo/Live";
import Scene from "./scene/Scene";
import s from "./home.module.css";

/// A few words outlined and named the way Clipframes outlines the element under the pointer.
/// The outline and its label are the app's own (desktop/ui/style.css), at the app's own size.
function Pointed({ children, name }: { children: React.ReactNode; name: string }) {
  return (
    <span className={s.pointed}>
      {children}
      <span className={`cf ${s.outline}`} aria-hidden="true">
        <span className="highlight" style={{ inset: 0 }}>
          <span className="label">{name}</span>
        </span>
      </span>
    </span>
  );
}

/// The home page itself. `preview` is for the copy built to be looked at without a server
/// (scripts/build-preview.mjs), which has no star count to ask for.
export function HomePage({ stars, preview = false }: { stars: number | null; preview?: boolean }) {
  return (
    <>
      <Header stars={stars} refresh={!preview} here="/" home />
      <main className={s.main}>
        <div className={`${s.wrap} ${s.hero}`} data-walk="hero">
          <div className={s.headline}>
            <h1>
              <span>Point at <Pointed name={'Text "the thing"'}>the thing</Pointed></span> <span>you want changed.</span>
            </h1>
          </div>
          <div className={s.say}>
            <p className={s.lede} data-walk="line">Clipframes tells Claude Code or Codex exactly which thing you mean.<span className={s.desk}> It is free, for Mac and Windows.</span></p>
            <span className={s.desk}><DownloadOne place="hero" /></span>
          </div>
          <Scene />
        </div>
      </main>
      <Footer />
      <Live />
    </>
  );
}
