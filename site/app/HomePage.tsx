// The home page itself. It is apart from page.tsx so the preview build can draw it too
// (scripts/build-preview.mjs).
//
// The order is the order a visitor's questions come in (design/critique-2.md): what it is and
// what the agent gets, in the first screen; why that matters; that it picks the right one of
// several; that it works in every app; the other two tools; living with it; how it differs
// from what they use today; and what to know before downloading, beside the download.

import Footer from "./Footer";
import Header from "./Header";
import { CopyLink, DownloadButtons } from "./DownloadButton";
import type { Place } from "./DownloadButton";
import HistoryDemo from "./demo/HistoryDemo";
import Live from "./demo/Live";
import Prompt from "./demo/Prompt";
import Stage from "./demo/Stage";
import Recording from "./Recording";
import { MAKER_URL, REPO_URL, REQUIREMENTS, VERSION } from "@/lib/site";
import s from "./home.module.css";

/// What each demo's example ends up copying (desktop/ui/web.ts writes these words). A demo's
/// prompt shows its line faintly until the demo has copied something itself.
const COPIES = {
  hero: '[Button "New invoice" (#new-invoice .btn.btn-primary), under heading "Invoices" in Google Chrome "Invoices": make this green]',
  picks: `[Clipframes: 3 things in Google Chrome "Invoices"]
1. Button "New invoice" (#new-invoice .btn.btn-primary), under heading "Invoices": make this green
2. Text "$3,120" (#overdue-total), under heading "Invoices": too alarming, use the normal text colour
3. Text "Paid" (#invoice-table .badge.paid), 2nd of 4 on the page, under heading "Invoices": make this one grey`,
  area: '[Screenshot (1.png) in Google Chrome "Invoices": put more space between these]',
  clip: '[Screen clip, 3 s, 12 frames (1/) in Google Chrome "Invoices": the menu opens far from the button]',
};

/// One line of the copied text taken apart, with what each part is for.
const PARTS: [part: string, says: string][] = [
  ['Text "Paid"', "What the thing is and what it says."],
  ["(#invoice-table .badge.paid)", "Its id and classes in your code. The agent searches your project for these. Clipframes reads them in browsers and in desktop apps built with Electron."],
  ["2nd of 4 on the page", "Which one it is, when several read the same."],
  ['under heading "Invoices"', "The heading it sits under."],
  ["make this one grey", "What you typed."],
];

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

/// The way to get the app: the two downloads, or on a phone, which cannot run it, the link to
/// open on a computer.
function Download({ place }: { place: Place }) {
  return (
    <div className={s.get} data-walk="download">
      <div className={`${s.buttons} ${s.desk}`}><DownloadButtons place={place} /></div>
      <div className={`${s.buttons} ${s.phone}`}>
        <CopyLink place={place} big />
      </div>
      <p className={`${s.meta} ${s.desk}`}>Free · Version {VERSION} · {REQUIREMENTS}</p>
      <p className={`${s.meta} ${s.phone}`}>Clipframes is free and runs on Mac and Windows. Open the link on your computer to download it.</p>
    </div>
  );
}

/// The home page itself. `preview` is for the copy built to be looked at without a server
/// (scripts/build-preview.mjs), which has no star count to ask for.
export function HomePage({ stars, preview = false }: { stars: number | null; preview?: boolean }) {
  return (
    <>
      <Header stars={stars} refresh={!preview} />
      <main className={s.main}>
        <div className={`${s.wrap} ${s.hero}`} data-walk="hero">
          <div className={s.heroText}>
            <div className={s.headline}>
              <h1>
                <span>Point at <Pointed name={'Text "the thing"'}>the thing</Pointed></span> <span>you want changed.</span>
              </h1>
            </div>
            <div className={s.pitch}>
              <p className={s.lede}>
                Clipframes is a free app for Mac and Windows. It tells Claude Code or Codex exactly which thing on your
                screen you mean.
              </p>
              <ol className={s.steps}>
                <li>Press <code className={s.inline}>Ctrl+Shift+Space</code> in any app.</li>
                <li>Click the thing and type what should change.</li>
                <li>Paste into your agent.</li>
              </ol>
              <div className={s.heroGet}>
                <Download place="hero" />
                <p className={s.assure}>
                  Your captures stay on your computer. <a href="#get">What it asks for and what it sends</a>
                </p>
              </div>
            </div>
          </div>
          <div className={s.bench}>
            <Stage
              id="hero"
              size="hero"
              invite
              label="A made-up invoicing app with the Clipframes bar at the bottom. The New invoice button is outlined and named, and a comment box under it says: make this green."
              hint="This is the real Clipframes bar on a made-up app. Click anything in the window."
              touchHint="This is the real Clipframes bar on a made-up app. Tap anything in the window."
            />
            <Prompt id="hero" example={COPIES.hero} />
          </div>
        </div>

        <section id="why" className={`${s.wrap} ${s.section} ${s.why}`}>
          <div className={s.said}>
            <p>
              When you build with an agent, the hard part is often <strong>saying which thing you mean</strong>. &ldquo;The blue
              button at the top&rdquo; can match five buttons, and you may not know what the button is called in the code.
            </p>
            <p>
              Clipframes reads it from the screen. <strong>Your agent gets the name the code uses</strong>, a picture and your
              comment, in one short text that you paste.
            </p>
          </div>
          <div className={s.anatomy} data-walk="anatomy">
            <h2>What is in the text</h2>
            <dl>
              {PARTS.map(([part, says]) => (
                <div key={part}>
                  <dt>{part}</dt>
                  <dd>{says}</dd>
                </div>
              ))}
            </dl>
            <p className={s.under}>
              In the app the text also names a notes file in <code className={s.inline}>~/Clipframes</code> that holds a
              picture of each thing. The agent opens the file when it needs to see what you saw.
            </p>
          </div>
        </section>

        <section id="which" className={`${s.wrap} ${s.section} ${s.split}`}>
          <div className={s.text}>
            <h2>It says which one you mean</h2>
            <p>
              A page often has several things that read the same. In this one, four invoices say Paid. When you pick one
              of them, the line says &ldquo;2nd of 4 on the page&rdquo; and names the heading above it, so the agent changes that
              one and leaves the other three alone.
            </p>
            <p data-walk="test result">
              In a test on small projects, agents changed the right element in 24 of 24 tries when the line had this
              wording, and in 18 of 24 tries when it did not.
            </p>
            <Prompt id="picks" example={COPIES.picks} />
          </div>
          <div className={s.visual}>
            <Stage
              id="picks"
              size="tall"
              label="Three things picked in the invoicing app, each with a number: the New invoice button, the overdue total and one of the Paid badges. The bar says 3 copied."
              hint="Click a few things and write a comment for each. One of the Paid badges shows it best."
              touchHint="Tap a few things and write a comment for each. One of the Paid badges shows it best."
            />
          </div>
        </section>

        <section id="apps" className={`${s.wrap} ${s.section}`}>
          <div className={s.text}>
            <h2>It works in every app on your computer</h2>
            <p>
              Clipframes is an app on your computer, so you add nothing to your project. It works on your web app in a
              browser, on desktop apps built with Electron or Tauri, and on native apps.
            </p>
            <p>
              In browsers and most desktop apps it reads the element&apos;s real name. In apps that draw their own interface,
              like some terminals and canvas apps, it reads the text near where you clicked.
            </p>
          </div>
          <div className={`${s.visual} ${s.desk}`}>
            <Recording name="desktop" label="A recording of Clipframes on Windows, 31 seconds long. A web app is open in Chrome on the left and Claude Code on the right. Two things are picked, and the text is pasted into Claude Code." />
          </div>
          <div className={`${s.visual} ${s.phone} ${s.closeups}`}>
            <Recording name="comment" label="A recording of Clipframes on Windows. The New invoice button is picked in Chrome and a comment is typed." />
            <Recording name="paste" label="The same recording a moment later. The text is pasted into Claude Code." />
          </div>
        </section>

        <section id="area" className={`${s.wrap} ${s.section} ${s.split}`}>
          <div className={s.text}>
            <h2>Drag over an area when it is easier to show</h2>
            <p>
              Choose the area tool in the bar and drag over part of the screen. Clipframes saves a picture of the area
              with your comment. You can mix areas, elements and clips in one go.
            </p>
            <Prompt id="area" example={COPIES.area} />
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

        <section id="clip" className={`${s.wrap} ${s.section} ${s.split}`}>
          <div className={s.text}>
            <h2>Record what happens when you click around</h2>
            <p>
              Some problems only show up in use: a menu that opens in the wrong place, or a button that does nothing.
              Choose the clip tool, drag over the area, do the thing and press Stop. The agent gets the frames in order
              and a list of every click you made.
            </p>
            <Prompt id="clip" example={COPIES.clip} />
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

        <section id="tab" className={`${s.wrap} ${s.section} ${s.split}`}>
          <div className={s.text}>
            <h2>It is there when you come back</h2>
            <p>
              Clipframes starts with your computer and shows nothing until you need it. When you come back to an app or
              a site where you used it, a small tab appears at the bottom of the screen. Click the tab to open the bar.
              The pin in the bar turns the tab off for that place.
            </p>
          </div>
          <div className={s.visual}>
            <Stage
              id="tab"
              opens={false}
              size="short"
              label="The invoicing app with Clipframes closed. A small black tab with the Clipframes mark sits at the bottom of the page."
              hint="Click the black tab at the bottom of the window to open the bar."
              touchHint="Tap the black tab at the bottom of the window to open the bar."
            />
          </div>
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

        <section id="compare" className={`${s.wrap} ${s.section} ${s.compare}`}>
          <h2>How it differs from what you may use today</h2>
          <div className={s.others}>
            <div>
              <h3>A screenshot</h3>
              <p>
                A screenshot shows the agent what you see. It does not say what the thing is called in the code, so the
                agent has to find it from the picture, and when four badges look the same it has to guess which one you
                meant. Clipframes writes the name, the id and classes and which one it is as text, and keeps a picture
                beside it for the agent to open.
              </p>
            </div>
            <div>
              <h3>A toolbar in your project</h3>
              <p>
                Agentation is a toolbar you install in a React web app, and it works inside that app. Clipframes is an
                app on your computer. It works in every app and site without adding anything to a project, and it can
                also record clips.
              </p>
            </div>
            <div>
              <h3>A browser extension</h3>
              <p>
                An extension works in the pages of the browser it is installed in. Clipframes works there too, and also
                in desktop apps: Electron, Tauri and native ones.
              </p>
            </div>
          </div>
        </section>

        <section id="get" className={`${s.wrap} ${s.section} ${s.end}`}>
          <div className={s.endGet}>
            <h2>Try it on your own app.</h2>
            <Download place="end" />
          </div>
          <dl className={s.facts} data-walk="facts">
            <div>
              <dt>Price</dt>
              <dd>Clipframes is free.</dd>
            </div>
            <div>
              <dt>Runs on</dt>
              <dd>A Mac with macOS 12 or later, on Apple Silicon or Intel, or a PC with Windows 10 or 11.</dd>
            </div>
            <div>
              <dt>Agents</dt>
              <dd>Claude Code, Codex, and any other agent you can paste text into.</dd>
            </div>
            <div>
              <dt>Size</dt>
              <dd>
                The download is about 5 MB for Mac and about 2 MB for Windows. On Windows it uses 22 to 26 MB of memory
                while it waits, and almost no processor time.
              </dd>
            </div>
            <div>
              <dt>Install</dt>
              <dd>
                On a Mac, unzip the download and move Clipframes to Applications. On Windows, run the installer. It
                updates itself from then on.
              </dd>
            </div>
            <div>
              <dt>Permissions</dt>
              <dd>
                A Mac asks for two on first use: Accessibility, to read which element you pointed at, and Screen
                Recording, to take pictures and clips. Windows asks for none.
              </dd>
            </div>
            <div>
              <dt>Your captures</dt>
              <dd>
                They stay on your computer. Each one is a folder in <code className={s.inline}>~/Clipframes</code> with the
                pictures, any clip frames and the notes.md your agent reads.
              </dd>
            </div>
            <div>
              <dt>What it sends</dt>
              <dd>
                Clipframes goes online for two things. It checks GitHub for a new version. It also sends anonymous counts
                of what is used and reports of errors, which never include what is on your screen, what you picked or
                what you wrote. Settings has a switch that turns them off, and the <a href="/privacy">privacy page</a> lists
                every event.
              </dd>
            </div>
            <div>
              <dt>Made by</dt>
              <dd>
                <a href={MAKER_URL}>Emil Wagman</a>. The source is on <a href={REPO_URL}>GitHub</a>.
              </dd>
            </div>
          </dl>
        </section>
      </main>

      <Footer />
      <Live />
    </>
  );
}
