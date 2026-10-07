import Header from "./Header";
import DownloadButton from "./DownloadButton";
import Film from "./Film";
import Output from "./Output";
import { MAKER_URL, REPO_URL } from "@/lib/site";
import s from "./home.module.css";

export default function Home() {
  return (
    <>
      <Header />
      <main className={s.main}>
        <div className={`${s.col} ${s.intro}`}>
          <h1>Point at the thing you want changed.</h1>
          <p>
            <strong>Clipframes tells your agent exactly what it is.</strong> When you build with Claude Code or Codex,
            the hard part is often saying which thing you mean. &ldquo;The blue button at the top&rdquo; can match five
            buttons, and you may not know what the button is called in the code. Clipframes lets you point at it instead.
            It copies a short reference with a screenshot and the element&apos;s name in the code, and you paste that into
            your agent.
          </p>
          <p>
            It works in every app on your Mac: your web app in Chrome or Safari, desktop apps built with Electron or Tauri,
            and native Mac apps. You don&apos;t add anything to your project. When something only goes wrong as you click
            around, you can record a clip of it.
          </p>
          <DownloadButton />
        </div>

        <section id="element" className={s.section}>
          <div className={s.col}>
            <h2>Point at one thing <span className={s.keys}>⌃⇧1</span></h2>
            <p>
              Press the shortcut, or Element in the Clipframes bar, and click a button, a card or a menu. In a browser,
              Clipframes reads the name the code uses for it, its id and classes, so the agent can find it in your project.
            </p>
          </div>
          <Film label="Clipframes capturing the New invoice button in a web app" />
          <Output />
        </section>

        <section id="screenshot" className={s.section}>
          <div className={s.col}>
            <h2>Show an area <span className={s.keys}>⌃⇧2</span></h2>
            <p>
              Drag over part of the screen. Clipframes saves the picture and lists the things inside it, so the agent knows
              which parts you mean.
            </p>
          </div>
          <Film label="Clipframes capturing an area of a web app" />
          <Output />
        </section>

        <section id="clip" className={s.section}>
          <div className={s.col}>
            <h2>Show what happens <span className={s.keys}>⌃⇧3</span></h2>
            <p>
              Some problems only show up when you click around: a menu that opens in the wrong place, or a button that does
              nothing. Press the shortcut, do the thing, and press it again. The agent gets the video, a set of frames and a
              list of every click.
            </p>
          </div>
          <Film label="Clipframes recording a clip of clicks in a web app" />
          <Output />
        </section>

        <section id="paste" className={s.section}>
          <div className={s.col}>
            <h2>Then paste it</h2>
            <p>
              Every capture puts one line on your clipboard. Paste it into Claude Code or Codex and say what you want
              changed. The agent opens the notes file and sees what you saw.
            </p>
          </div>
          <Film label="Pasting a Clipframes reference into Claude Code" />
        </section>

        <section id="try" className={s.section}>
          <div className={s.col}>
            <h2>Try it on this page</h2>
            <p>
              Once Clipframes is installed, press <code className={s.inline}>⌃⇧1</code> and click one of these. Then paste
              somewhere to see what your agent would get.
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
          </div>
        </section>

        <section id="private" className={s.section}>
          <div className={s.col}>
            <h2>Your captures stay on your Mac</h2>
            <p>
              Each capture is a folder in <code className={s.inline}>~/Clipframes</code> with the screenshot, any video and
              frames, and the notes.md your agent reads. Nothing is uploaded.
            </p>
            <p>
              Clipframes goes online only to check for updates, and it installs them for you. You can turn that off in
              Settings.
            </p>
          </div>
        </section>

        <section id="faq" className={`${s.section} ${s.faq}`}>
          <div className={s.col}>
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
            <h3>Is it open source?</h3>
            <p>The source is on <a href={REPO_URL}>GitHub</a>.</p>
          </div>
        </section>

        <div className={`${s.col} ${s.end}`}>
          <h2>Try it on your own app.</h2>
          <DownloadButton />
        </div>
      </main>

      <footer className={s.footer}>
        <span>Made by <a href={MAKER_URL}>Emil Wagman</a></span>
        <a href={REPO_URL}>Source on GitHub</a>
      </footer>
    </>
  );
}
