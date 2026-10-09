import Header from "./Header";
import { DownloadButtons } from "./DownloadButton";
import { MAKER_URL, REPO_URL, REQUIREMENTS, VERSION } from "@/lib/site";
import { starCount } from "@/lib/github";
import s from "./home.module.css";

/// A hand-drawn loop around a few words, stretched to fit them.
function Circled({ children }: { children: React.ReactNode }) {
  return (
    <span className={s.circled}>
      {children}
      <svg viewBox="0 0 400 160" preserveAspectRatio="none" aria-hidden="true">
        <path d="M70 30 C150 2 330 8 370 58 C398 104 300 140 190 144 C90 146 18 122 22 80 C26 46 80 26 132 22" />
      </svg>
    </span>
  );
}

function Download({ note }: { note?: string }) {
  return (
    <div className={s.get}>
      <DownloadButtons />
      {note ? (
        <span className={s.note}>
          {note}
          <svg viewBox="0 0 60 36" aria-hidden="true"><path d="M56 22 C42 30 22 30 6 16M16 10 L5 16 L13 26" /></svg>
        </span>
      ) : (
        <span>Version {VERSION} · {REQUIREMENTS}</span>
      )}
    </div>
  );
}

/// The app's real interface drawn over the Northwind demo page (demo/tools/render-shots.mjs).
function Shot({ src, alt, caption }: { src: string; alt: string; caption?: string }) {
  return (
    <figure className={s.figure}>
      <img className={s.shot} src={`/shots/${src}`} alt={alt} width={1440} height={900} loading="lazy" />
      {caption && <figcaption>{caption}</figcaption>}
    </figure>
  );
}

const ref = `[Clipframes: 2 things in Google Chrome "Invoices". Read ~/Clipframes/2026-10-09_11-42-30/notes.md]
1. Button "New invoice" (#new-invoice .btn.btn-primary): make this green
2. Text "$3,120" (#overdue-total .stat.overdue): too alarming, use the normal text colour`;

export default async function Home() {
  const stars = await starCount(3600);
  return (
    <>
      <Header stars={stars} />
      <main className={s.main}>
        <div className={`${s.col} ${s.intro}`}>
          <h1>Point at <Circled>the thing</Circled> you want changed.</h1>
          <p className={s.lede}>
            Clipframes is a small app for Mac and Windows, for building with Claude Code and Codex. Point at a button,
            drag an area or record a clip, say what you want, and your agent knows exactly what you mean.
          </p>
          <Download note="it's free" />
          <p className={s.meta}>Version {VERSION} · {REQUIREMENTS}</p>
        </div>

        <Shot src="element.jpg" alt="Clipframes highlighting the New invoice button in a web app, with its name and selector in a label above it" />

        <section className={`${s.col} ${s.section}`}>
          <p>
            When you build with an agent, the hard part is often saying which thing you mean. &ldquo;The blue button at the
            top&rdquo; can match five buttons, and you may not know what the button is called in the code. Clipframes
            copies a short reference with the element&apos;s name in the code, a picture of it and your comment, and you
            paste that into your agent.
          </p>
          <p>
            It works in every app on your computer: your web app in a browser, desktop apps built with Electron or Tauri,
            and native apps. You don&apos;t add anything to your project.
          </p>
        </section>

        <section id="element" className={`${s.col} ${s.section}`}>
          <h2>Point at things and say what you want</h2>
          <p>
            Press <code className={s.inline}>Ctrl+Shift+Space</code> from any app to open the Clipframes bar, then click a
            button, a card or a menu. A box opens next to it for your comment. Click the next thing and comment on that
            too. In a browser, Clipframes reads the name the code uses for each one, its id and classes, so the agent can
            find it in your project. This is what lands on your clipboard:
          </p>
          <div className={s.ref}><code>{ref}</code></div>
        </section>
        <Shot src="pick.jpg" alt="Two things picked in a web app with Clipframes, each numbered, with a comment box open under the second one and the bar saying 2 things copied" caption="Everything you pick is on your clipboard straight away. Keep going, or paste." />

        <section id="screenshot" className={`${s.col} ${s.section}`}>
          <h2>Show an area</h2>
          <p>
            Choose Area in the bar and drag over part of the screen. Clipframes saves the picture and your comment about
            it. You can mix areas, elements and clips in the same go.
          </p>
        </section>
        <Shot src="screenshot.jpg" alt="An area of a web app being selected with Clipframes, the rest of the screen dimmed" />

        <section id="clip" className={`${s.col} ${s.section}`}>
          <h2>Show what happens</h2>
          <p>
            Some problems only show up when you click around: a menu that opens in the wrong place, or a button that does
            nothing. Choose Clip, drag over the area, do the thing, and press Stop. The agent gets the frames in order and
            a list of every click you made.
          </p>
        </section>
        <Shot src="clip.jpg" alt="Clipframes recording an area of a web app, with the bar showing the time and a Stop button" />

        <section id="paste" className={`${s.col} ${s.section}`}>
          <h2>Then paste it</h2>
          <p>
            Paste into Claude Code or Codex. The agent gets one short list of what you picked and what you said, and opens
            the notes file to see what you saw.
          </p>
        </section>
        <Shot src="history.jpg" alt="The Clipframes history: past captures with small pictures, each with a Copy button" caption="Everything you captured stays in History, ready to copy again." />

        <section id="there" className={`${s.col} ${s.section}`}>
          <h2>It is there when you need it</h2>
          <p>
            Clipframes starts with your computer and stays out of the way. When you come back to an app or a site where
            you used it, a small tab appears at the bottom of the screen. Click it to open the bar. You can turn the tab
            off for any place with the pin in the bar.
          </p>
        </section>

        <section id="try" className={`${s.col} ${s.section}`}>
          <h2>Try it on this page</h2>
          <p>
            Once Clipframes is installed, press <code className={s.inline}>Ctrl+Shift+Space</code> and click one of
            these. Then paste somewhere to see what your agent would get.
          </p>
          <div className={s.try}>
            <div className={s.tryRow}>
              <button id="save-changes" className={`btn btn-primary ${s.primary}`}>Save changes</button>
              <button id="cancel" className="btn">Cancel</button>
              <input id="project-name" type="text" placeholder="Project name" aria-label="Project name" />
            </div>
            <div className={s.card} id="invoice-card">
              <div><b>Invoice INV-1042</b><span>Maersk Line · due Sep 12</span></div>
              <span className={s.badge}>Overdue</span>
            </div>
          </div>
        </section>

        <section id="private" className={`${s.col} ${s.section}`}>
          <h2>Your captures stay on your computer</h2>
          <p>
            Each capture is a folder in <code className={s.inline}>~/Clipframes</code> with the pictures, any clip frames,
            and the notes.md your agent reads. Nothing is uploaded.
          </p>
          <p>Clipframes goes online only to check for updates, and it installs them for you.</p>
        </section>

        <section id="faq" className={`${s.col} ${s.section} ${s.faq}`}>
          <h2>Questions</h2>
          <h3>How is this different from Agentation?</h3>
          <p>
            Agentation is a toolbar you install in a React web app, and it works inside that app. Clipframes runs on your
            computer, so it works in every app without adding anything to your project, and it can also record clips.
          </p>
          <h3>Which apps does it read?</h3>
          <p>
            In browsers and most desktop apps, Clipframes reads the element&apos;s real name. In apps that draw their own
            interface, like some terminals and canvas apps, it reads the text near where you clicked instead.
          </p>
          <h3>What does it need?</h3>
          <p>
            A Mac with macOS 12 or later, on Apple Silicon or Intel, or a PC with Windows 10 or 11. On a Mac it asks for
            Accessibility, to read which element you pointed at, and Screen Recording, to take pictures and clips.
            Windows asks for nothing.
          </p>
          <h3>Can I see the code?</h3>
          <p>The source is on <a href={REPO_URL}>GitHub</a>.</p>
        </section>

        <div className={`${s.col} ${s.end}`}>
          <h2>Try it on your own app.</h2>
          <Download />
        </div>
      </main>

      <footer className={s.footer}>
        <span>Made by <a href={MAKER_URL}>Emil Wagman</a></span>
        <a href={REPO_URL}>Source on GitHub</a>
      </footer>
    </>
  );
}
