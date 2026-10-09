// Checks that a web page words "which one" and "under what heading" exactly as the app's core
// does (src-tauri/src/element/locate.rs and mod.rs have the same cases). Run: pnpm test:ui
import assert from "node:assert/strict";
import { headingText, ordinal, whereabouts } from "../ui/whereabouts.ts";

// A page as a test writes it. `el("a", "GitHub")` is <a>GitHub</a>.
const el = (tag, ...inside) => {
  const children = inside.filter((c) => typeof c !== "string");
  return {
    tagName: tag.toUpperCase(),
    children,
    attributes: {},
    get textContent() {
      return inside.map((c) => (typeof c === "string" ? c : c.textContent)).join(" ");
    },
    getAttribute(name) {
      return this.attributes[name] ?? null;
    },
  };
};
const reads = (e) => `${e.tagName} "${e.textContent}"`;
const like = (target) => (e) => e.children.length === 0 && reads(e) === reads(target);
const roomy = { nodes: 1000, ms: 5000 };

assert.deepEqual([1, 2, 3, 4, 11, 12, 13, 21, 22, 23, 101, 111, 112].map(ordinal), ["1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "22nd", "23rd", "101st", "111th", "112th"]);
assert.equal(headingText("  Try it on\n your own app. "), "Try it on your own app.");
const long = headingText("word ".repeat(30));
assert.equal([...long].length, 60);
assert.ok(long.endsWith("word…"));

// The website: a top bar with a download link, a section with its own heading and the same
// link again, and a questions section.
const [top, github1, bottom, button, github2] = [el("a", "Download for Windows"), el("a", "GitHub"), el("a", "Download for Windows"), el("button", "Download for Windows"), el("a", "GitHub")];
const questionsHeading = el("div", "Questions");
questionsHeading.attributes.role = "heading";
const bar = el("div", top, github1);
const tryIt = el("section", el("h2", "Try it on your own app."), bottom, button);
const questions = el("section", questionsHeading, github2, el("div"));
const page = el("body", bar, el("h1", "Point at what you want changed."), tryIt, questions);

assert.deepEqual(whereabouts(page, bottom, like(bottom), roomy), ["2nd of 2 on the page", 'under heading "Try it on your own app."']);
assert.deepEqual(whereabouts(page, top, like(top), roomy), ["1st of 2 on the page"], "above every heading");
assert.deepEqual(whereabouts(page, github2, like(github2), roomy), ["2nd of 2 on the page", 'under heading "Questions"'], "role=heading counts");
assert.deepEqual(whereabouts(page, button, like(button), roomy), ['under heading "Try it on your own app."'], "the only one of its kind has no number");
assert.deepEqual(whereabouts(page, bar, null, roomy), [], "nothing before it, nothing in it");
assert.deepEqual(whereabouts(page, tryIt, null, roomy), ['under heading "Point at what you want changed."']);
assert.deepEqual(whereabouts(page, questionsHeading, like(questionsHeading), roomy), [], "a heading is not said to be under another");

// A section with no heading before it is named by the one it holds.
const held = el("section", el("div", el("h2", "Questions")));
assert.deepEqual(whereabouts(el("body", held, el("section", el("h2", "Later"))), held, null, roomy), ['with heading "Questions"']);
const empty = el("div");
assert.deepEqual(whereabouts(el("body", empty, el("h2", "Later")), empty, null, roomy), []);

// When the budget runs out nothing is said: a count that stopped early would be a wrong count.
assert.deepEqual(whereabouts(page, top, like(top), { nodes: 5, ms: 5000 }), []);
assert.deepEqual(whereabouts(page, bottom, like(bottom), { nodes: 1000, ms: 0 }, (() => { let t = 0; return () => (t += 1); })()), [], "no time at all");
assert.deepEqual(whereabouts(page, el("a", "GitHub"), null, roomy), [], "an element that is not on the page");

console.log("whereabouts: all checks pass");
