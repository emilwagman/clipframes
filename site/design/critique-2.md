# Critique of iteration 1, and the plan for iteration 2

Written 2026-10-10, before any of iteration 2 was built. The before/after table at the end was
filled in afterwards.

How it was measured: `scripts/walk.mjs` loads the built site in headless Chrome at 1440×900 and
at 390×844 (touch, 2x), takes one picture per screen and a contact sheet, and prints where every
heading, demo, copied-text block and download link is, in pixels and in screens (pixels divided
by the height of the screen). A "screen" below always means that. Pictures of iteration 1 are in
`critique-2/before/`, pictures of the reference sites in `critique-2/refs/`.

Each claim is marked (O) observed in a measurement or picture, or (I) inferred.

The visitor: someone who builds with Claude Code or Codex, saw a post on X, and tapped the link.
They give the page a few seconds to say what it is, and two or three screens to convince them.

## 1. The visitor's questions, in the order they have them

Where iteration 1 answers each one, at 1440 and at 390.

| # | Question | Where at 1440 | Where at 390 | Form |
|---|---|---|---|---|
| 1 | What is this? | 0.22 (headline), 0.47 (subhead) | 0.17, 0.22 | Text |
| 2 | Is it for me? | 0.47: the subhead names Claude Code and Codex. The reason to care ("saying which thing you mean") is at 1.9 | 0.25, and 1.3 | Text |
| 3 | What does it do on my screen? | 0.82: the demo starts there, and 170 px of its 620 px are in the first screen. The whole demo is in view after scrolling 0.6 screens | 0.79, 180 px of 540 px in the first screen | Live demo that plays by itself |
| 4 | What does my agent receive? | 3.72 (the copied text under the second demo), again at 6.64 | 3.70, 6.78 | Text block fed by the demo above it |
| 5 | Does it work in my app, native apps too? | 2.0 (second paragraph of the grey text), detail at 9.57 in Questions | 1.7, 10.21 | Text |
| 6 | Does it work with my agent? | 0.47 (named), 6.60 ("Then paste it") | 0.25, 6.52 | Text |
| 7 | Which systems? | 0.70: the small line under the buttons, and the button labels | 0.49 | Text |
| 8 | What does it cost? | 0.70: one word, "Free", in the same small line | 0.49 | Text |
| 9 | Is it safe, what leaves my computer? | 8.84 | 9.17 | Text, details behind /privacy |
| 10 | How do I install, what will it ask for? | 9.78 (third item in Questions) | 10.47 | Text |
| 11 | How heavy is it? | Nowhere | Nowhere | |
| 12 | Why not a screenshot? | Nowhere | Nowhere | |
| 13 | Why not Agentation or a browser extension? | 9.57 (first item in Questions) | 9.91 | Text |
| 14 | Who made it, is it kept up? | 10.6 (footer: maker), 0.70 (version), header (GitHub) | 11.4 | Text |

All positions (O), from `critique-2/before/map.json`.

What the table says:

- The first three questions are answered early, in words. The answer in pictures (question 3)
  starts below the fold.
- Question 4 is the one that decides it for this audience, and it is 3.7 screens down.
- Questions 9 and 10 are what a careful person checks right before pressing Download. They sit
  eight to ten screens below the download buttons they belong to.
- Questions 11 and 12 have no answer at all. "Why not a screenshot" is the first objection
  anyone who already pastes screenshots into Claude Code will have (I).

## 2. Every interactive piece

| Piece | Where (1440 / 390) | Can you tell before touching? | First hover or click | Touch | What it proves |
|---|---|---|---|---|---|
| Hero demo | 0.82 / 0.79 | Partly. A drawn pointer plays an example, which shows it is alive. The sentence that says "click anything" is a 14.5 px grey caption under the stage, at 1.55 screens (O). The cursor over the stage is the default arrow (O). | The example stops and the real outline and name tag follow the pointer at once. A click opens the comment box. | Works: a tap picks, a swipe scrolls (O, check-demos). | That Clipframes names what is under the pointer. It does not show what the agent gets. |
| Second demo (several picks) | 2.81 / 2.75 | Same as the hero. | Same. | Works. | Numbered picks. The copied text is below it, 813 px further down, so the two are never in one screen at 1440 (O). |
| Copied text under it | 3.72 / 3.70 | Not interactive, but live: it changes when you pick. | Nothing. | n/a | The text for the agent. On a phone it showed "Nothing is picked yet." after the example was cut short by scrolling (O, screen 4 of the phone sheet), which reads as broken. |
| Area demo | 4.40 / 4.47 | Same as the hero. | Dragging draws a box and opens the comment box. | Works by a sideways drag, which the caption says, under the stage. | That an area can be sent. It does not show what the agent gets. |
| Clip demo | 5.67 / 5.66 | Same. | Drag, the bar shows Recording, Stop. | Works. | That a clip can be recorded. It does not show what the agent gets. |
| "Then paste it" prompt | 6.64 / 6.78 | It looks like a white card. It is fed by whatever was picked last, which nobody would guess. | Nothing. | n/a | The pasted text, far from the demos that made it. |
| History panel | 7.06 / 7.22 | Yes: it has Copy buttons. | Copy puts the text on the clipboard. | Works. | That captures are kept. |
| Tab demo | 8.12 / 8.51 | The tab is a 48 px square in a 420 px stage. | Click opens the bar. | Works. | That the tab opens the bar. |
| "Play the example again" | under each stage | Yes, a link. Hidden while the example plays. | Replays. | Works. | |

Five full copies of the same invoicing page appear one after another (screens 1, 3, 5, 6 and 9 of
the 1440 sheet). At contact-sheet size the page reads as the same picture five times (O). Each
stage is 620 px tall plus the browser bar, which is 73% of a 900 px screen, so every demo costs
most of a screen and the sentence it proves has scrolled away by the time the demo is in view
(O for the sizes, I for the effect).

## 3. The "this is for me" moment

Today it has no single place. The parts are spread over 3.7 screens:

- 0.22: the headline with "the thing" outlined and tagged. It shows the idea in the app's own
  graphics, and it is the strongest thing on the first screen (I).
- 0.82 to 1.5: the demo names a button. Interesting, though not yet useful: a visitor sees a
  label, and has no reason yet to think their agent will do better with it.
- 1.9: "The blue button at the top can match five buttons." This is the sentence the visitor
  recognises from their own week (I). It comes after the demo it should set up.
- 3.72: the line of text the agent gets. This is the proof, and most visitors have left by now.

Honest look at the hero at 1440×900 (`before/1440-first.jpg`):

- The headline is 88 px and takes 200 px. It says what you do, and the outlined words show how.
  It works. It does not say what you get.
- The subhead is three lines at 23 px and carries five ideas: what it is, the systems, the
  agents, the three tools, and the result. The result ("your agent knows exactly what you mean")
  is a claim; nothing on the first screen shows it.
- Two black download buttons at 548 px are the largest dark shapes on the screen. The page asks
  for the download before it has shown the product. A visitor from X is not ready for it (I).
- The demo starts at 734 px. In the first screen you see the browser bar, the page title and
  the top edge of three cards. The example plays there anyway: the comment box opens 3.9 s after
  load (O) and is cut off by the bottom of the screen.
- The right 40% of the first screen, beside the headline and subhead, is empty (O).

Where it should happen: in the first screen, in under ten seconds, without reading instructions.
The visitor should see something get outlined and named, see the comment typed, and see the
exact line land in an agent's prompt beside it. Then the same thing should happen to whatever
they point at themselves.

## 4. Scroll depth

Assume most visitors stop within two to three screens.

| Zone | What is there in iteration 1 (1440) |
|---|---|
| Screen 1 | Headline, subhead, download buttons, 170 px of the demo |
| Screens 2 to 3 | The rest of the hero demo, the grey "why" paragraphs, the heading of the second section and the top of the second demo |
| Past screen 3 | The text the agent gets (3.72), areas (4.1), clips (5.3), paste (6.6), history (7.1), the tab (7.8), privacy (8.8), Agentation (9.6), requirements and permissions (9.8), the second download (10.3) |

Important and buried: the text the agent gets, the "which one of four" wording (the part with
a measured result behind it: 24 of 24 against 18 of 24), privacy, permissions, and the
comparison with the tools a visitor already knows.

Fine where it is, or lower: history and the tab. They matter after someone has installed it.

The page is 10.66 screens at 1440 and 11.53 at 390. Four of the six reference sites are longer,
so length is not the fault. The order is.

## 5. Navigation

- Header: the name, GitHub with the star count, one Download button. It stays on screen while
  scrolling at both sizes (O). On a phone the button starts a download the phone cannot use.
- Footer: the maker, Privacy, Source on GitHub.
- `/privacy` holds the full list of events the app and the site send. That is the right place
  for two long tables. The short answer a visitor needs ("your captures stay on your computer;
  anonymous counts, with an off switch") is on the home page, though at 8.84 screens.
- `/download` picks the file for the visitor's system.
- GitHub holds the README, which answers install and "what leaves your computer" sooner than
  the site does.

Nothing a visitor needs in order to decide is only behind a link. Everything is on the home
page, in the wrong order. Nothing on the home page needs a page of its own: it is one small
app, and the reference sites that use many pages do so for pricing plans, teams and docs, which
Clipframes does not have. So: stay one page, keep `/privacy` for the detail, and move the short
answers up beside the download.

The header does not need section links. The page will have about eight sections, and a visitor
who wants one thing (privacy, requirements) should find it by the download buttons, where the
question comes up.

## 6. How comparable sites handle the same questions

Measured the same way on 2026-10-10, headless Chrome, 1440×900 and 390×844. All six loaded.
agentation.dev redirects to www.agentation.com. Pictures in `critique-2/refs/`. Everything in
the table is (O) unless marked.

| Site | Product visual in the first screen (top edge, share of the screen) | First thing that moves or responds (screens down, kind) | Install | Price | Privacy | Action stays while scrolling | Action repeated at the end | Phone |
|---|---|---|---|---|---|---|---|---|
| Agentation | Yes, 203 px, 12% | 0.23, a scripted animation. The real toolbar is live on the page, with "Try it" at 1.40 | A command to copy at 56 px | 2.57 of 2.99 | Not on the page | Sidebar yes, the command no | No | "Desktop only" banner; no live toolbar |
| Raycast | No (a decorative canvas) | 1.70, a scripted mock with a clickable dock | Download button at 797 px, requirement line under it | 15.9 of 17.8 | Footer link | Yes | Yes | No download button at all |
| Linear | Yes, 527 px, 38% | 0.59, live interface that also plays by itself | Sign up (web app) | Nav link | Footer link | Yes | Yes | Sign up |
| Cursor | Yes, 351 px, 55% | 0.39, a scripted demo built in the page | Download button at 251 px | Nav link | Footer link | Yes | Yes | Button becomes "Get started" |
| CleanShot | Yes, 638 px, 14% | 0.71, an autoplaying video of 0.74 MB | Button to the pricing page | Pricing page | Footer link | Yes | Yes | Same button |
| Screen Studio | Yes, 588 px, 27% | 0.65, a video of 3.2 MB that starts on the first scroll | Download button at 454 px, requirement line under it | 24.9 of 27.5 | A whole section, at 20.2 | Yes | Footer only | "Email me a link" |

Rules that hold on at least three of them:

1. The product is in the first screen. Five of six; top edges between 0.23 and 0.71. (O)
   Iteration 1 is at 0.82 with 19% of the screen, the latest of all but Raycast.
2. Headline, at most one or two sentences, then the action, all above or beside the visual.
   Headlines are 4 to 9 words. (O) Iteration 1 fits this except for the five-idea subhead.
3. When the product is an interface, the demo is that interface built in the page. Agentation,
   Linear, Cursor, Raycast. Video heroes belong to the two apps whose output is video. (O)
   Iteration 1 already does this, and with the real components.
4. The demo runs by itself. Nobody labels the hero "try it"; the cues are a drawn pointer, a
   play icon, a tooltip. (O) None of the five puts the result of the demo beside it, because
   their products do not hand something to another tool. Ours does, so this is where we can do
   something they do not (I).
5. The system is named inside the button and the requirement sits right under it. Raycast,
   Screen Studio, Cursor, CleanShot. (O) Iteration 1 does this.
6. The header stays and keeps the action. Five of six. (O) Iteration 1 does this.
7. Price is a nav link or sits in the last 15% of the page. (O) Ours is one word, "Free",
   which is an advantage worth saying early.
8. Privacy is a footer link on four of six. Screen Studio alone gives it a section, with two
   columns: what stays on the Mac, what leaves only when you choose. (O) Clipframes reads the
   screen, so it needs the Screen Studio treatment more than the others do (I).
9. Some proof arrives within 1.7 screens on four of six: a counter, a quote, logos. (O) We
   have no users to count and will not invent any. The one honest piece of proof we own is the
   agent test (24 of 24 against 18 of 24), and it is not on the site at all.
10. The page ends by repeating the hero's action. Four of six. (O) Iteration 1 does.
11. Phones: three of the four sites with a desktop download remove or replace the button.
    Screen Studio's "Email me a link" is the only one that helps the visitor. (O) We will not
    collect email addresses, so the equivalent is a button that copies the link.
12. Comparisons stay out of the main flow: footer links, one paragraph on Agentation ("Without
    Agentation, you'd have to describe the element…"), one FAQ item on Screen Studio. (O)

Taken as grammar only. Nothing of their look is used: the light ground, the black type, the
one orange and the app's own outlines stay as they are.

## 7. Problems, most costly first, with the fix

1. **The proof is 3.7 screens down.** The line the agent receives is the product, and it first
   appears at 3344 px. Fix: an agent's prompt sits in the hero, joined to the demo. The example
   types its comment and the line appears in the prompt as it is typed. Whatever the visitor
   picks lands in the same prompt. Target: visible in the first screen at 1440×900 and
   1280×720, and the visitor's own line within ten seconds of arriving.
2. **The product starts below the fold.** 170 of 620 px in the first screen, with 40% of that
   screen empty beside the headline. Fix: the demo moves up into the first screen. Whether it
   sits beside the headline, under it, or before it is a matter of taste, so all three are
   built (`?hero=a`, `b`, `c`) and compared on measurements.
3. **Nothing in view says the demo can be touched.** The invitation is a grey caption under
   the stage at 1.55 screens. Fix: one plain sentence above the stage, in the first screen,
   saying this is the real bar and to click anything in the window.
4. **What a careful visitor checks before downloading is eight to ten screens from the
   button.** Privacy at 8.84, permissions at 9.78, size nowhere. Fix: a short line by the hero
   buttons (free, the systems, captures stay on the computer) that links down to one section
   beside the second download, which answers price, systems, size, permissions, what leaves
   the computer, which agents, updates and who makes it, in a sentence each.
5. **The demos do not show their result.** Area and clip demos end with a comment box; the
   text is somewhere else on the page. Fix: every demo has its own prompt next to it, showing
   what that demo puts on the clipboard. Each section then proves its own sentence.
6. **The order follows the app's features, and the visitor's questions come in another
   order.** Fix: hero (what, for whom, what the agent gets), then why it matters and what is
   in the line, then "which one of four" with the test result, then every app (with a real
   recording), then area, clip, the tab and history, then the comparison, then the facts and
   the download.
7. **"Why not a screenshot" is not answered, and Agentation is the fourth-last thing on the
   page.** Fix: one section with two plain paragraphs, one per alternative, saying what each
   is and what Clipframes does differently, with no knocking.
8. **The same full-size invoicing page five times.** Fix: after the hero, each stage is
   smaller and shares its row with the text and its prompt, so a section fits in one screen
   and no two screens look alike.
9. **A phone gets download buttons it cannot use, twice, 124 px tall together, above the
   demo.** Fix: on a phone the buttons are replaced by one that copies the download link, with
   a sentence saying Clipframes runs on Mac and Windows, and the demo moves above it.
10. **The agent test is not on the site.** Fix: say it, with its limits, in the "which one"
    section.
11. **Nothing shows the app outside a web page.** Every demo is a browser frame, which is the
    opposite of "works in every app" (I). Fix: the real recording of the app on Windows, with
    Chrome and Claude Code side by side, in the "every app" section, loaded only when it comes
    near the screen. (There is no recording of a native app yet. That one would be better.)
12. **The subhead carries five ideas.** Fix: two sentences, what it is and what your agent
    gets.

Kept as it is: the headline and its outlined words, the light ground and black type, the
orange as the only accent, the real interface in every demo, the examples that play by
themselves, the made-up invoicing app, touch handling, the privacy page, the footer.

## 8. What was built, and the same walk again

Iteration 2 is on branch `site-3`. Measured the same way, on the built site, 2026-10-10.
Pictures in `critique-2/after/`.

What changed, against the problems above:

1. The hero holds the demo and, joined to it, a black prompt titled "Pasted into your agent".
   The example picks the New invoice button and types "make this green", and the line appears
   in the prompt as it is typed, about five seconds after the page loads. A visitor's own pick
   lands in the same prompt and on their clipboard.
2. The demo is whole in the first screen at 1440×900 (404 of 404 px) and at 1280×720 the part
   the example uses is. On a phone the prompt comes first and 291 px of the demo follow it in
   the first screen.
3. One sentence above the stage says it is the real bar and to click anything in the window.
4. A line by the first download says captures stay on the computer and links to the last
   section, where nine short answers sit beside the second download.
5. Every demo has its own prompt, which shows what that demo copied. Before a demo has copied
   anything its prompt shows the example's text faintly, so it is never empty.
6. The sections follow the questions: hero, why and what is in the text, which one of four
   (with the test result), every app (with the real recording), area, clip, the tab, history,
   how it differs, the answers and the download.
7. "How it differs from what you may use today" answers the screenshot, Agentation and a
   browser extension in a paragraph each.
8. After the hero each stage is 500 to 600 px and shares its row with its words and prompt.
9. On a phone the downloads are a button that copies the link to the page, with one sentence
   saying where Clipframes runs. Nothing is sent anywhere.
10. The test result is in the "which one" section, with its limits ("on small projects").
11. The recording of the app on Windows, Chrome beside Claude Code, is in the "every app"
    section. It is 462 kB and nothing of it is fetched until it is within 700 px of the screen.
    A phone gets two close-ups from the same recording (118 kB and 68 kB).
12. The subhead is two sentences.

The page went from 10.66 to 8.76 screens at 1440, and from 11.53 to 13.46 at 390 (a phone now
gets a prompt under every demo and the nine answers as a list).

### The hero's three layouts

A matter of taste, so all three are on the real page: `?hero=a`, `?hero=b`, `?hero=c`.
Pictures: `after/hero-a-1440x900-played.jpg` and its two neighbours.

| | a: demo beside the headline | b: headline above the demo | c: demo first |
|---|---|---|---|
| Headline size at 1440 | 59 px | 77 px | 69 px |
| Demo width at 1440 | 667 px, the made-up app without its sidebar | 808 px, with its sidebar | 808 px |
| Agent's prompt | under the demo | beside the demo, as tall as it | beside the demo |
| In the first screen at 1440×900 | everything, with three numbered steps | everything | everything; the headline is at 660 px |
| In the first screen at 1280×720 | the demo whole, the prompt whole | the demo down to under the comment box, the prompt whole | the same, and the headline is cut off |
| Reads as | a page about an app | iteration 1, with the demo moved up into view | a demo with a caption |

**b is the one the page is built with.** It keeps what was liked in iteration 1 (the large
headline on the left edge, the demo nearly as wide as the page) and is the arrangement four of
the six reference sites use. a is the safest on small laptops and says the most in words. c
shows the product soonest and says what it is last, which is the wrong order for someone who
arrived from a post and does not know the name yet.

### Before and after

Screens down to the answer, at 1440 and at 390. "Nowhere" means not on the page.

| # | Question | Before 1440 | After 1440 | Before 390 | After 390 |
|---|---|---|---|---|---|
| 1 | What is this? | 0.22 | 0.16 | 0.17 | 0.12 |
| 2 | Is it for me? (the agents named) | 0.47 | 0.13 | 0.25 | 0.25 |
| 2 | Is it for me? (the problem named) | 1.9 | 1.10 | 1.3 | 1.70 |
| 3 | What does it do on my screen? | 0.82, 27% of the demo in the first screen | 0.45, all of it in the first screen | 0.79, 33% in the first screen | 0.66, 73% in the first screen |
| 4 | What does my agent receive? | 3.72 | 0.75 | 3.70 | 0.44 |
| 5 | Does it work in my app, native apps too? | 2.0 | 2.73, with a recording at 3.14 | 1.7 | 1.21 ("in any app"), section at 4.71 |
| 6 | Does it work with my agent? | 0.47 | 0.13, again at 7.84 | 0.25 | 0.25, again at 12.11 |
| 7 | Which systems? | 0.70 | 0.13 and 0.28 | 0.49 | 0.25 and 1.48 |
| 8 | What does it cost? | 0.70 (one word) | 0.13 (in the subhead) | 0.49 | 0.25 |
| 9 | What leaves my computer? | 8.84 | 0.31 (one line and a link), in full at 8.21 | 9.17 | 1.54 (one line and a link), in full at 12.75 |
| 10 | How do I install, what will it ask for? | 9.78 | 8.10, linked from 0.31 | 10.47 | 12.57, linked from 1.54 |
| 11 | How heavy is it? | Nowhere | 7.90 | Nowhere | 12.24 |
| 12 | Why not a screenshot? | Nowhere | 7.23 | Nowhere | 10.70 |
| 13 | Why not Agentation or an extension? | 9.57 | 7.23 | 9.91 | 11.04 |
| 14 | Who made it? | 10.6 | 8.46 | 11.4 | 13.16 |
| | Is there proof it helps? (the agent test) | Nowhere | 2.10 | Nowhere | 3.27 |

Read honestly:

- Questions 1 to 4, 6, 7 and 8 are now answered in the first screen at 1440. On a phone the
  agent's text is in the first screen and the demo is three quarters in it.
- Questions 9 and 10 have a one-line answer and a link in the first screen at 1440 (and at 1.5
  screens on a phone), and the full answer beside the second download. The full answer is
  still at the end of the page. It was put there because that is where the second download
  is; a visitor who wants it sooner has the link.
- Questions 12 and 13 are after the product sections, as on the reference sites. They did not
  move far at 1440 and are further down on a phone than before.
- On a phone three rows got worse in screens (the problem named, 10 and 13), because the page
  is longer there.

### Rules the page now follows

- The first screen shows the product working and the thing it produces, and says in one
  sentence what a visitor can do with it.
- A demo and what it proves are in view together: its words beside it, its text for the agent
  beside or under it.
- One question per section, in the order visitors ask them.
- What someone checks before installing is said in one line by the first download and in
  full beside the last one.
- Nothing above the fold is a video. Recordings are fetched when they come near the screen.
- A phone is not offered a download.
- `node scripts/walk.mjs` prints where everything is; run it after any change to the order.
