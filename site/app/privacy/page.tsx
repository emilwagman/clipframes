import type { Metadata } from "next";
import Footer from "../Footer";
import Header from "../Header";
import { analyticsOn } from "@/lib/analytics";
import { starCount } from "@/lib/github";
import { REPO_URL, SITE_NAME } from "@/lib/site";
import s from "./privacy.module.css";

export const metadata: Metadata = {
  title: `Privacy · ${SITE_NAME}`,
  description: "What the Clipframes app sends, and what this website collects.",
};

/// Every event the app sends: the table in desktop/TELEMETRY.md, which is the source. Change both together.
const APP: [events: string[], when: string, sent: string][] = [
  [["app_started"], "The app starts", "how (login, hand, update), first run or not, whether the shortcut could be taken, whether start at login is on"],
  [["round_opened"], "The bar opens", "how (shortcut, tab, tray, launch, other), whether the bar was kept warm, milliseconds until clicks were captured and until all windows were up, number of displays"],
  [["round_blocked"], "The bar opens but cannot pick", "how it was asked for (as for round_opened), why (permission, screen, picker)"],
  [["pick_added"], "Something is picked", "kind (element, area, clip), whether a picture was taken; for an element whether it had a selector, a name and a web address (yes or no each); for an area its size in pixels; for a clip its seconds, frames and number of clicks"],
  [["pick_removed"], "Remove is pressed", "nothing"],
  [["round_closed"], "The bar closes", "number of picks of each kind, how many had a comment, seconds the bar was open"],
  [["history_copied"], "Copy is pressed in History", "number of picks copied"],
  [["history_deleted"], "A capture is deleted in History", "nothing"],
  [["place_auto_set"], "The pin in the bar is pressed", "on or off"],
  [["shortcut_changed"], "A new shortcut is saved", "nothing"],
  [["update_found"], "A new version is found", "its version number"],
  [["$exception"], "Something failed: a screenshot, an update, the picker, a crash, or an error in a window's own code", "the kind (capture, update, picker, panic, ui), a message, and the place in Clipframes' code (a function, or a file name and line; for a window also which window)"],
];

/// Every event this website sends (lib/analytics.ts and where it is called).
const SITE: [events: string[], when: string, sent: string][] = [
  [["$pageview"], "A page of this site is opened", "nothing more"],
  [["download_clicked"], "A download button is pressed", "which system (mac, windows, or auto for the button at the top) and where on the page the button is (header, hero, end)"],
  [["link_copied"], "On a phone, the button that copies this site's address is pressed", "where on the page the button is (header, hero, end)"],
  [["demo_started"], "You take over one of the demos on the home page", "which demo (hero, picks, area, clip, tab)"],
  [["demo_finished"], "You pick something in a demo, or open the bar from the tab", "which demo"],
  [["reference_shown"], "The text a demo copied is on screen in the prompt beside it", "which demo, and whether the text came from the example or from your own pick (example, visitor). The text itself is not sent"],
  [["reference_copied"], "You copy the text from one of those prompts", "nothing more"],
  [["recording_played"], "A recording of the app on the home page starts playing", "which recording (desktop, comment, paste)"],
];

function Events({ rows }: { rows: [string[], string, string][] }) {
  return (
    <table className={s.events}>
      <thead>
        <tr><th>Event</th><th>When</th><th>What is sent with it</th></tr>
      </thead>
      <tbody>
        {rows.map(([events, when, sent]) => (
          <tr key={events[0]}>
            <td data-label="Event">{events.map((event, i) => <span key={event}>{i > 0 && ", "}<code>{event}</code></span>)}</td>
            <td data-label="When">{when}</td>
            <td data-label="What is sent with it">{sent}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export default async function Privacy() {
  const stars = await starCount(3600);
  return (
    <>
      <Header stars={stars} />
      <main className={s.page}>
        <h1>Privacy</h1>
        <p className={s.lede}>
          Your captures stay on your computer. This page lists what the Clipframes app sends, and what this website
          collects.
        </p>

        <section>
          <h2>What the app sends</h2>
          <p>Clipframes goes online for two things.</p>
          <p>It checks GitHub for a new version.</p>
          <p>
            It sends anonymous counts of what is used and reports of errors, so problems can be found and fixed. It can
            be turned off in Settings (&ldquo;Share anonymous usage&rdquo;).
          </p>
          <p>
            It never sends what is on your screen, the names or text of what you pick, your comments, pictures, file
            paths, window titles, app names or site addresses.
          </p>
          <p>
            Every event carries: a random install number made on first run (it identifies a copy of the app, not a
            person), the app version, the operating system (<code>macos</code>, <code>windows</code>, <code>linux</code>) and
            the processor type, and the sender&apos;s name (<code>clipframes</code>). No person profile is made and no
            location is looked up from the address the event came from.
          </p>
          <p>
            Events wait up to ten seconds and leave in small batches. Turning the setting off also drops the ones still
            waiting.
          </p>
          <Events rows={APP} />
          <p>
            The message of an <code>$exception</code> is the error&apos;s text up to the first thing in it that looks like a
            file path or an address, and at most 300 characters. For an update it is only the name of the kind of
            error (<code>Minisign</code>, <code>Io</code>, …), never its text. An update check that failed because the
            computer was offline, or because no release exists yet, is not reported at all.
          </p>
          <p className={s.note}>
            The code is <a href={`${REPO_URL}/blob/main/desktop/src-tauri/src/telemetry.rs`}>src-tauri/src/telemetry.rs</a>. A
            build without a reporting key set at build time has nowhere to send to, sends nothing, and does not show
            the setting.
          </p>
        </section>

        <section>
          <h2>What this website collects</h2>
          {analyticsOn() ? (
            <>
              <p>
                This website sends anonymous counts of visits and of a few clicks to PostHog, on servers in the EU. It
                sets no cookies and keeps nothing in your browser. It records no sessions and makes no person profile,
                and it asks for no location to be looked up from the address an event came from.
              </p>
              <p>
                Every event carries: a random number made new each time a page is loaded, the path of the page, and
                the kind of browser, system and device. Nothing you pick or type in the demos is sent: no element
                names, no comments, no copied text.
              </p>
              <Events rows={SITE} />
            </>
          ) : (
            <p>This website does not count visits or clicks. It sets no cookies and keeps nothing in your browser.</p>
          )}
          <p>
            The demos on the home page run in your browser. When you pick something in one, the text is put on your
            clipboard, as the app would do. It is not sent anywhere.
          </p>
          <p>
            The site is hosted by Vercel, which receives the address each request comes from, as any web host does.
            The download buttons send you to GitHub, which serves the files.
          </p>
        </section>
      </main>
      <Footer />
    </>
  );
}
