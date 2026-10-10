# Design rulebook for the site

What the owner has said about the site, dated, in his words, and the rule taken from it. Add
to this every time he states or changes a preference. The measurements behind each round are
in `critique-2.md` and `critique-3.md`.

## 2026-10-10, after iteration 1

"Take your professional web design lens and look critically at the website. Where is
information presented? When is it interactive? How far do people scroll? Should information be
hidden under navigation to pages? Think about the questions that a user will have, and where
are those answered. And where will you become convinced that this is for you?"

Rules (my reading of it):
- The first screen shows the product working and the thing it produces.
- A demo and what it proves are in view together.
- One question per section, in the order visitors ask them. `node scripts/walk.mjs` prints
  where everything is; run it after any change to the order.
- What someone checks before installing is said in full beside the last download.
- Nothing above the fold is a video. A phone is not offered a download.

## 2026-10-10, after iteration 2

"I see where this is getting at. but this is tooo much shit all at once."

Rules (my reading of it):
- The first screen is one thing: the headline, one short line, and the demo. Count what is in
  it outside the demo's window. Iteration 2 had 13; keep it at 6 or fewer.
- Adding an answer to the first screen is not free. An answer that is not needed to
  understand the product goes right under the first screen, said quietly, or to the end.
- A result is shown after the act that made it, as part of the same object. It is not a
  second panel that is always there.
- The demo invites by what it does (the bar's own words, the outline under the pointer). It
  does not get a sentence of instruction.
- One primary action in the first screen.
- A visitor's first pick in a demo starts a clean round.

## 2026-10-10, after iteration 3 (iterations 2 and 3 are set aside; the work restarts from iteration 1)

"I dont think execution was good. good thoughts though. really think about design. we also need
to show more relatable shit. Claude code, codex. things that these people will relate to. use
cases, motion design, coding, web design, etc. like lots of things. and nothing ever shows it
working in desktop apps. honestly from what I see, this iteration is just a total downgrade.
this iteration was just a bunch of information printed everywhere, so much shit to read,
nothing feels instantly relatable to me at all."

And: "we need to somehow answer the questions people will have. like how does this compare to
agentation. we dont have to solve everything on the home page though. we can solve it with
extra pages. like a nav bar or a side wise nav". The bar for launch: the app and the website
are to be "absolute top tier".

Rules (my reading of it):
- Show, don't print. A question is answered by something that happens on screen. A paragraph,
  a panel of text or a list of facts is not an answer on the home page.
- Show the visitor's own setup: the thing they are building beside the agent's terminal, with
  Claude Code and Codex both seen. Recognition comes before explanation.
- Show real kinds of work, several of them: a web app, a desktop app, motion, a game. One scene
  that changes, not a section each.
- A desktop app must be seen being picked. A browser frame alone says "web only".
- Fewer words than the version before, never more. Count them (`node scripts/words.mjs`).
  Iteration 1 had 979 words on the home page and 70 in the first screen; iteration 2 had 1313
  and 119. The pilot's first screen has 43.
- The questions that need words get their own pages, reached from a few plain words in the top
  bar. Those pages show first too, and use short sentences.
- Anything said about another product is checked against that product's own site, dated, or
  left out.
- The critique's findings still hold (the proof must be in the first screen); it was the answer
  to them, more text, that was wrong.
