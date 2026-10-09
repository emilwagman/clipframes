import Header from "./Header";
import { DownloadButtons } from "./DownloadButton";
import { Copied, Pasted } from "./demo/Copied";
import HistoryDemo from "./demo/HistoryDemo";
import Live from "./demo/Live";
import Stage from "./demo/Stage";
import { MAKER_URL, REPO_URL, REQUIREMENTS, VERSION } from "@/lib/site";
import { starCount } from "@/lib/github";
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

function Download() {
  return (
    <div className={s.get}>
      <div className={s.buttons}><DownloadButtons /></div>
      <p className={s.meta}>Free · Version {VERSION} · {REQUIREMENTS}</p>
    </div>
  );
}

export default async function Home() {
  const stars = await starCount(3600);
  return (
    <>
      <Header stars={stars} />
      <main className={s.main}>
        <div className={`${s.wrap} ${s.intro}`}>
          <h1>
            <span>Point at <Pointed name={'Text "the thing"'}>the thing</Pointed></span> <span>you want changed.</span>
          </h1>
          <p className={s.lede}>
            Clipframes is a small app for Mac and Windows, for building with Claude Code and Codex. Point at a button,
            drag an area or record a clip, say what you want, and your agent knows exactly what you mean.
          </p>
          <Download />
          <div className={s.visual}>
            <Stage
              id="hero"
              label="A made-up invoicing app with the Clipframes bar at the bottom. The New invoice button is outlined and named, and a comment box under it says: make this green."
              hint="This page is a made-up invoicing app with the real Clipframes bar on it. Move your pointer over it and click anything."
              touchHint="This page is a made-up invoicing app with the real Clipframes bar on it. Tap anything in it."
            />
          </div>
        </div>

        <section className={`${s.wrap} ${s.section} ${s.why}`}>
          <p>
            When you build with an agent, the hard part is often <strong>saying which thing you mean</strong>. &ldquo;The blue button at the
            top&rdquo; can match five buttons, and you may not know what the button is called in the code. Clipframes
            copies a short reference with the element&apos;s name in the code, a picture of it and your comment, and you
            paste that into your agent.
          </p>
          <p>
            <strong>It works in every app on your computer</strong>: your web app in a browser, desktop apps built with Electron or Tauri,
            and native apps. You don&apos;t add anything to your project.
          </p>
        </section>

        <section className={`${s.wrap} ${s.section}`}>
          <div className={s.text}>
            <h2>Point at things and say what you want</h2>
            <p>
              Press <code className={s.inline}>Ctrl+Shift+Space</code> from any app to open the Clipframes bar, then click a
              button, a card or a menu. A box opens next to it for your comment. Click the next thing and comment on that
              too. In a browser, Clipframes reads the name the code uses for each one, its id and classes, so the agent can
              find it in your project.
            </p>
          </div>
          <div className={s.visual}>
            <Stage
              id="picks"
              label="Two things picked in the invoicing app, each with a number: the New invoice button and the overdue total. The bar says 2 copied."
              hint="Click a few things and write a comment for each."
              touchHint="Tap a few things and write a comment for each."
            />
          </div>
          <div className={s.lands}>
            <p>This is what lands on your clipboard:</p>
            <Copied />
          </div>
        </section>

        <section className={`${s.wrap} ${s.section}`}>
          <div className={s.text}>
            <h2>Show an area</h2>
            <p>
              Choose the area tool in the bar and drag over part of the screen. Clipframes saves the picture and your
              comment about it. You can mix areas, elements and clips in the same go.
            </p>
          </div>
          <div className={s.visual}>
            <Stage
              id="area"
              tool="area"
              label="An area dragged around the three totals in the invoicing app, with a comment box under it that says: put more space between these."
              hint="Drag over any part of the page."
              touchHint="Drag sideways over any part of the page."
            />
          </div>
        </section>

        <section className={`${s.wrap} ${s.section}`}>
          <div className={s.text}>
            <h2>Show what happens</h2>
            <p>
              Some problems only show up when you click around: a menu that opens in the wrong place, or a button that does
              nothing. Choose the clip tool, drag over the area, do the thing, and press Stop. The agent gets the frames in
              order and a list of every click you made.
            </p>
          </div>
          <div className={s.visual}>
            <Stage
              id="clip"
              tool="clip"
              label="A screen clip of the top of the invoicing app, where the Export menu has opened far from its button. The comment box says: the menu opens far from the button."
              hint="Drag over an area to start recording, click around in the page, then press Stop."
              touchHint="Drag sideways over an area to start recording, tap around in the page, then press Stop."
            />
          </div>
        </section>

        <section className={`${s.wrap} ${s.section} ${s.split}`}>
          <div className={s.text}>
            <h2>Then paste it</h2>
            <p>
              Paste into Claude Code or Codex. The agent gets one short list of what you picked and what you said, and opens
              the notes file to see what you saw.
            </p>
          </div>
          <div className={s.visual}><Pasted /></div>
        </section>

        <section id="history" className={`${s.wrap} ${s.section} ${s.split}`}>
          <div className={s.text}>
            <h2>Copy an earlier capture again</h2>
            <p>Everything you captured stays in History, ready to copy again. The clock in the bar opens it.</p>
          </div>
          <div className={s.visual}>
            <HistoryDemo />
            <p className={s.under}>What you pick on this page is added to the top of this list.</p>
          </div>
        </section>

        <section className={`${s.wrap} ${s.section}`}>
          <div className={s.text}>
            <h2>It is there when you need it</h2>
            <p>
              Clipframes starts with your computer and stays out of the way. When you come back to an app or a site where
              you used it, a small tab appears at the bottom of the screen. Click it to open the bar. You can turn the tab
              off for any place with the pin in the bar.
            </p>
          </div>
          <div className={s.visual}>
            <Stage
              id="tab"
              opens={false}
              size="short"
              label="The invoicing app with Clipframes closed. A small black tab with the Clipframes mark sits at the bottom of the page."
              hint="Click the tab at the bottom of the page to open the bar."
              touchHint="Tap the tab at the bottom of the page to open the bar."
            />
          </div>
        </section>

        <section className={`${s.wrap} ${s.section}`}>
          <div className={s.text}>
            <h2>Your captures stay on your computer</h2>
            <p>
              Each capture is a folder in <code className={s.inline}>~/Clipframes</code> with the pictures, any clip frames,
              and the notes.md your agent reads. Nothing is uploaded.
            </p>
            <p>Clipframes goes online only to check for updates, and it installs them for you.</p>
          </div>
        </section>

        <section className={`${s.wrap} ${s.section} ${s.faq}`}>
          <h2>Questions</h2>
          <div className={s.answers}>
            <div>
              <h3>How is this different from Agentation?</h3>
              <p>
                Agentation is a toolbar you install in a React web app, and it works inside that app. Clipframes runs on your
                computer, so it works in every app without adding anything to your project, and it can also record clips.
              </p>
            </div>
            <div>
              <h3>Which apps does it read?</h3>
              <p>
                In browsers and most desktop apps, Clipframes reads the element&apos;s real name. In apps that draw their own
                interface, like some terminals and canvas apps, it reads the text near where you clicked instead.
              </p>
            </div>
            <div>
              <h3>What does it need?</h3>
              <p>
                A Mac with macOS 12 or later, on Apple Silicon or Intel, or a PC with Windows 10 or 11. On a Mac it asks for
                Accessibility, to read which element you pointed at, and Screen Recording, to take pictures and clips.
                Windows asks for nothing.
              </p>
            </div>
            <div>
              <h3>Can I see the code?</h3>
              <p>The source is on <a href={REPO_URL}>GitHub</a>.</p>
            </div>
          </div>
        </section>

        <div className={`${s.wrap} ${s.end}`}>
          <h2>Try it on your own app.</h2>
          <Download />
        </div>
      </main>

      <footer className={s.footer}>
        <span>Made by <a href={MAKER_URL}>Emil Wagman</a></span>
        <a href={REPO_URL}>Source on GitHub</a>
      </footer>
      <Live />
    </>
  );
}
