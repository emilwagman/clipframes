import Header from "./Header";
import { DownloadIcon } from "./DownloadButton";
import { DOWNLOAD_PATH, MAKER_URL, REPO_URL, REQUIREMENTS, VERSION } from "@/lib/site";
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
      <a className={`${s.pill} ${s.big}`} href={DOWNLOAD_PATH}><DownloadIcon />Download for Mac</a>
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

const ref = '[Element: Button "New invoice" (#new-invoice .btn-primary) in Chrome "Invoices". Read ~/Clipframes/2026-10-07_11-42-30/notes.md]';

export default function Home() {
  return (
    <>
      <Header />
      <main className={s.main}>
        <div className={`${s.col} ${s.intro}`}>
          <h1>Point at <Circled>the thing</Circled> you want changed.</h1>
          <p className={s.lede}>
            Clipframes is a Mac app for building with Claude Code and Codex. Point at a button, drag an area or record a
            clip, and your agent knows exactly what you mean.
          </p>
          <Download note="it's free" />
          <p className={s.meta}>Version {VERSION} · {REQUIREMENTS}</p>
        </div>

        <section className={`${s.col} ${s.section}`}>
          <p>
            When you build with an agent, the hard part is often saying which thing you mean. &ldquo;The blue button at the
            top&rdquo; can match five buttons, and you may not know what the button is called in the code. Clipframes
            copies a short reference with a screenshot and the element&apos;s name in the code, and you paste that into
            your agent.
          </p>
          <p>
            It works in every app on your Mac: your web app in Chrome or Safari, desktop apps built with Electron or Tauri,
            and native Mac apps. You don&apos;t add anything to your project.
          </p>
        </section>

        <section id="element" className={`${s.col} ${s.section}`}>
          <h2>Point at one thing</h2>
          <p>
            Press <code className={s.inline}>⌃⇧Space</code> from any app to open the Clipframes bar, then press 1 for
            Element and click a button, a card or a menu. In a browser, Clipframes reads the name the code uses for it, its
            id and classes, so the agent can find it in your project. This is what lands on your clipboard:
          </p>
          <div className={s.ref}><code>{ref}</code></div>
        </section>

        <section id="screenshot" className={`${s.col} ${s.section}`}>
          <h2>Show an area</h2>
          <p>
            Press 2 and drag over part of the screen. Clipframes saves the picture and lists the things inside it, so the
            agent knows which parts you mean. Every capture is kept in the library, ready to copy again.
          </p>
        </section>

        <section id="clip" className={`${s.col} ${s.section}`}>
          <h2>Show what happens</h2>
          <p>
            Some problems only show up when you click around: a menu that opens in the wrong place, or a button that does
            nothing. Press 3, do the thing, and press <code className={s.inline}>⌃⇧Space</code> to stop. The agent gets the
            video, a set of frames and a list of every click.
          </p>
        </section>

        <section id="paste" className={`${s.col} ${s.section}`}>
          <h2>Then paste it</h2>
          <p>
            Every capture puts one line on your clipboard. Paste it into Claude Code or Codex and say what you want changed.
            The agent opens the notes file and sees what you saw.
          </p>
        </section>

        <section id="try" className={`${s.col} ${s.section}`}>
          <h2>Try it on this page</h2>
          <p>
            Once Clipframes is installed, press <code className={s.inline}>⌃⇧Space</code>, then 1, and click one of these.
            Then paste somewhere to see what your agent would get.
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
          <h2>Your captures stay on your Mac</h2>
          <p>
            Each capture is a folder in <code className={s.inline}>~/Clipframes</code> with the screenshot, any video and
            frames, and the notes.md your agent reads. Nothing is uploaded.
          </p>
          <p>Clipframes goes online only to check for updates, and it installs them for you. You can turn that off in Settings.</p>
        </section>

        <section id="faq" className={`${s.col} ${s.section} ${s.faq}`}>
          <h2>Questions</h2>
          <h3>How is this different from Agentation?</h3>
          <p>
            Agentation is a toolbar you install in a React web app, and it works inside that app. Clipframes runs on your
            Mac, so it works in every app without adding anything to your project, and it can also record clips.
          </p>
          <h3>Which apps does it read?</h3>
          <p>
            In browsers and most desktop apps, Clipframes reads the element&apos;s real name. In apps that draw their own
            interface, like some terminals and canvas apps, it reads the text near where you clicked instead.
          </p>
          <h3>What does it need?</h3>
          <p>
            macOS 15 or later, on Apple Silicon or Intel. On first launch it asks for Screen Recording, to take screenshots
            and clips, and Accessibility, to read which element you pointed at.
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
