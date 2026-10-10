# Iteration 3: the first screen, with most of it taken away

2026-10-10. Emil on iteration 2's first screen: "I see where this is getting at. but this is
tooo much shit all at once". The direction stays (the product at work and what the agent gets,
at the top). This pass only subtracts, and only in the first screen. The sections under it are
as they were. Pictures in `critique-3/`; the quarter-size comparison is
`critique-3/squint-before-a-b-c.jpg`.

## What is in the first screen at 1440×900

Counted the way the feedback counted it: each separate thing the eye has to deal with.

| | Iteration 2 | a (built in) | b | c |
|---|---|---|---|---|
| Header: name, GitHub, Download | 3 | 3 | 3 | 3 |
| Headline with its outline | 1 | 1 | 1 | 1, on one line |
| One line saying what it is | 1 (two sentences) | 1 | 1 | |
| Download buttons | 2 | 1 | | |
| Version and systems line | 1 | | | |
| Privacy line with a link | 1 | | | |
| Sentence inviting a click | 1 | | | |
| The window | 1, with a sidebar and seven invoices | 1, no sidebar, two invoices | 1 | 1 |
| The agent's text | 1, a panel as tall as the window, always there | the window's foot, one line, there after the pick | foot, two lines | foot, one line |
| Caption under the agent's text | 1 | | | |
| "Play the example again" | 1, a link under the window | in the window's top bar, 12 px grey | same | same |
| **Things outside the window** | **13** | **6** | **5** | **4** |

The bar, the outline and the comment box are inside the window in every column and are the
point of it, so they are not counted against it. `scripts/check-demos.mjs` checks the list of
things outside the window for each layout at 1440×900 and 1280×720, before and after the
example has played.

## Where the rest went

- **Second download button:** gone from the first screen. The one button there is "Download
  Clipframes", which gives the file for the visitor's system (`/download`). Both systems are
  named as two links in the line under the first screen, and as buttons at the end.
- **Version, systems, "captures stay on your computer" and its link:** two quiet lines that
  start exactly one screen down (901 px at 1440×900), right under the hero. The full answers
  are still beside the last download.
- **The sentence inviting a click:** removed. The bar itself says "Click anything", and the
  outline follows the pointer the moment it is over the window.
- **The caption under the agent's text:** removed. The foot has four words above its text:
  "Pasted into your agent".
- **The tall black panel:** now the foot of the same window. It is closed until something is
  picked, opens to the height of its text (one line at 1200 px, two at 570 px, three on a
  phone), and grows only when more is picked. The hero is as tall as the screen so the foot
  opens into room that is already there; nothing under it moves.
- **The made-up app:** no sidebar and two invoices in the first screen. The demos further down
  keep the full app, because one of them needs the four Paid badges.
- **The three numbered steps** (old layout a): removed.
- **Phone:** headline, the one line, the window, and its foot. The copy-link button and its
  sentence start one screen down. The header keeps "Copy link".

## A fault that is fixed

In iteration 2 a visitor who clicked "New invoice" after the example got "2 things": the
example's pick and their own, with the example's comment on the wrong one. Now the visitor's
first pick in any demo takes the example's picks away first, so the text is what they did and
nothing else. If they instead type on in the comment box the example left open, that pick is
theirs and stays. Three checks cover it (mouse, touch, and going on with the example's comment).

## The three layouts (`?hero=a|b|c`)

- **a, built in.** The headline (82 px) with the line and one button beside it, above a window
  as wide as the page. This is the feedback's own list: the headline, one short line, the demo.
  It keeps the one sentence that tells someone from a post what this is and whether it is for
  them, and one action.
- **b.** The headline (70 px) and the line on the left, a 570 px window on the right. The
  calmest, with the most empty ground. No button; the header has one. The window is small
  enough that the made-up app loses its search field.
- **c.** The headline on one line (75 px) and the window. Four things besides the window. It
  is the strongest picture, and it does not say what Clipframes is until one screen down.

Squint test at quarter size: in a and c the headline and one window read as a single block; in
b they read as two, side by side. Iteration 2 reads as four (headline, text block with two
buttons, window, black panel).

## Least sure

- At 1280×720 the window is 340 px tall and the bar sits over the second invoice.
- "Download Clipframes" in the hero and "Download" in the header are the same link, 250 px
  apart. One of them may be one too many.
- Whether c's missing sentence matters as much as I think it does.
