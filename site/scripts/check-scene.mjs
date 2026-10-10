// Checks the first screen's scene the way a visitor meets it, with a mouse at 1440 px and a
// finger at 390 px: what is in the first screen; that every kind of work plays its example to
// the end in every arrangement of the scene (the pick, the text in the agent's prompt, the
// agent's answer, the changed thing); that the scene moves on by itself and stops when the
// visitor steps in; that a visitor's own pick is pasted and answered, and only what was really
// done is claimed; the real recording; a phone's sheet; and the Compare page. Headless.
//
//   npm run build && npx next start -p 5231 &
//   node scripts/check-scene.mjs [http://localhost:5231]
//
// Needs Google Chrome.
import assert from "node:assert/strict";
import { chromium } from "playwright-core";

const url = process.argv[2] ?? "http://localhost:5231";
const browser = await chromium.launch({ channel: "chrome", headless: true });
let failed = 0;

const ONLY = 'This demo only acts on a colour, "bigger", "smaller", "hide" and "rename to".';
/// What each kind of work's example ends with: the agent, what was pasted, the two lines of its answer, and how to tell the thing changed.
const CASES = {
  web: {
    agent: "claude", you: "❯", dot: "●",
    text: /^\[Button "New invoice" \(#new-invoice \.btn\.btn-primary\), under heading "Invoices" in Google Chrome "Invoices": make this black\]$/,
    did: "Update(src/components/InvoiceHeader.tsx)", says: "The New invoice button is black now.",
    changed: (page) => page.locator("[data-scene] #new-invoice").evaluate((el) => getComputedStyle(el).backgroundColor === "rgb(23, 24, 28)"),
  },
  desktop: {
    agent: "codex", you: "›", dot: "•",
    text: /^\[Button "Start" \(id=StartButton\) in Focus: make this green\]$/,
    did: "Edited MainWindow.xaml (+1 -1)", says: "The Start button is green now.",
    changed: (page) => page.locator("[data-scene] #StartButton").evaluate((el) => getComputedStyle(el).backgroundColor === "rgb(31, 157, 85)"),
  },
  motion: {
    agent: "claude", you: "❯", dot: "●",
    // A clip is as long as it took: two seconds with the example at its own pace, one when it is not acted out.
    text: /^\[Screen clip, [12] s, [48] frames \(1\/\) in Google Chrome "Launch film": the card overshoots, ease it out\]$/,
    did: "Update(src/scenes/Card.tsx)", says: "The card eases out now. The overshoot is gone.",
    changed: (page) => page.locator("[data-scene] #card").evaluate((el) => getComputedStyle(el).animationName.includes("card-eases")),
  },
  game: {
    agent: "codex", you: "›", dot: "•",
    text: /^\[Screenshot \(1\.png\) in Hopper: the health bar overlaps the score\]$/,
    did: "Edited src/hud.ts (+2 -2)", says: "The score sits under the health bar now.",
    // The score is drawn under the health bar: there is ink where there was sky.
    changed: (page) => page.locator("[data-scene] #view").evaluate((canvas) => {
      const ratio = canvas.width / canvas.clientWidth;
      const { data } = canvas.getContext("2d").getImageData(16 * ratio, 44 * ratio, 100 * ratio, 14 * ratio);
      let dark = 0;
      for (let i = 0; i < data.length; i += 4) if (data[i] < 60 && data[i + 1] < 60 && data[i + 2] < 60) dark += 1;
      return dark > 40;
    }),
  },
};
const ORDER = Object.keys(CASES);

async function session(name, options, body, path = "") {
  const context = await browser.newContext({ ...options, permissions: ["clipboard-read", "clipboard-write"] });
  const page = await context.newPage();
  const errors = [];
  page.on("console", (message) => message.type() === "error" && errors.push(message.text()));
  page.on("pageerror", (error) => errors.push(String(error)));
  page.on("response", (response) => response.status() >= 400 && errors.push(`${response.status()}: ${response.url()}`));
  await context.route((address) => address.origin !== new URL(url).origin, (route) => route.abort());
  const t = {
    page,
    box: (selector) => page.locator(selector).first().boundingBox(),
    /// The example for the kind of work on show has played to its end.
    played: (which) => page.waitForFunction((which) => { const scene = document.querySelector("[data-scene]"); return scene.dataset.scene === which && scene.dataset.playing === undefined && document.querySelector("[data-now] [data-line=says]") !== null; }, which, { timeout: 25000, polling: 100 }),
    /// What has been said in the agent's window since the example began: each line's mark and words.
    said: () => page.locator("[data-now] p").evaluateAll((lines) => lines.map((p) => [p.dataset.line, p.querySelector("i").textContent, p.querySelector("span").textContent])),
    /// The agent has said this many things in answer.
    answers: (count) => page.waitForFunction((count) => document.querySelectorAll("[data-now] [data-line=says]").length >= count, count, { timeout: 12000 }),
    at: async (selector, fx = 0.5, fy = 0.5) => { const box = await page.locator(`[data-scene] ${selector}`).boundingBox(); return { x: box.x + box.width * fx, y: box.y + box.height * fy }; },
    note: () => page.locator("[data-scene] .clipframes-web:not([data-still]) #comment"),
    bar: () => page.locator("[data-scene] .clipframes-web:not([data-still]) .bar"),
    clipboard: () => page.evaluate(() => navigator.clipboard.readText()),
    /// Points at something with the mouse, clicks it and saves a comment.
    async pick(selector, comment) {
      const target = await t.at(selector);
      await page.mouse.move(target.x - 50, target.y + 60);
      await page.mouse.move(target.x, target.y, { steps: 8 });
      await page.mouse.click(target.x, target.y);
      await t.note().waitFor();
      await page.keyboard.type(comment);
      await page.keyboard.press("Enter");
    },
    /// An example ended as it should for its kind of work.
    async ended(which) {
      const want = CASES[which];
      const said = await t.said();
      assert.equal(said.length, 3, JSON.stringify(said));
      assert.deepEqual(said.map(([who, mark]) => [who, mark]), [["you", want.you], ["did", want.dot], ["says", want.dot]]);
      assert.match(said[0][2], want.text);
      assert.deepEqual([said[1][2], said[2][2]], [want.did, want.says]);
      assert.equal(await page.locator("[data-scene] [data-agent]").getAttribute("data-agent"), want.agent);
      // The change is eased in, so it is given a moment.
      let changed = false;
      for (let tries = 0; tries < 15 && !changed; tries++) changed = (await want.changed(page)) || (await page.waitForTimeout(100), false);
      assert.equal(changed, true, `the ${which} did not change`);
      assert.equal(await page.locator(`[data-case="${which}"]`).getAttribute("aria-selected"), "true");
    },
  };
  try {
    await page.goto(url + path, { waitUntil: "networkidle" });
    await body(t);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false, "the page scrolls sideways");
    assert.deepEqual(errors, [], "the console has errors");
    console.log(`ok    ${name}`);
  } catch (error) {
    failed += 1;
    console.log(`FAIL  ${name}\n      ${String(error.message ?? error).split("\n").join("\n      ")}`);
  }
  await context.close();
}

const desktop = { viewport: { width: 1440, height: 900 } };
const phone = { viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 };
const overlap = (a, b) => a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;

for (const layout of ["", "a", "b", "c"]) {
  for (const [width, height] of [[1440, 900], [1280, 720]]) {
    await session(`desktop ${width}×${height}, scene ${layout || "as built"}: the first screen is the headline, one line, one download and the scene`, { viewport: { width, height } }, async (t) => {
      assert.equal(await t.page.evaluate(() => document.documentElement.dataset.sceneLayout), layout || "a");
      const desk = await t.box("[data-scene] [role=group]");
      const under = await t.box("[data-scene] figcaption");
      assert.ok(desk.y + desk.height <= height, `the scene ends at ${desk.y + desk.height}`);
      if (height >= 900) assert.ok(under.y + under.height <= height, `the kinds of work end at ${under.y + under.height}`);
      assert.equal(await t.page.locator("main h1:not(.northwind h1)").count(), 1);
      assert.deepEqual(await t.page.evaluate(() => [...document.querySelectorAll('a[href^="/download"]')].filter((a) => getComputedStyle(a).visibility !== "hidden" && a.getBoundingClientRect().width > 0).map((a) => a.textContent)), ["Download Clipframes"], "more than one download is showing");
      const words = await t.page.evaluate(() => [...document.querySelectorAll("header, main h1:not(.northwind h1), [data-walk=line], main a, [data-scene] figcaption")].map((el) => el.innerText).join(" ").split(/\s+/).filter(Boolean).length);
      assert.ok(words < 56, `${words} words outside the scene`);
      // The agent's window is the agent's: its name, its prompt, and what was said before, so it is not an empty black box.
      const term = await t.box("[data-scene] [data-agent]");
      const ink = await t.page.locator("[data-scene] [data-agent]").evaluate((el) => { const lines = [...el.querySelectorAll("p, [data-agent-prompt]")].filter((p) => p.getBoundingClientRect().height > 0); return lines.reduce((sum, p) => sum + p.getBoundingClientRect().height, 0); });
      assert.ok(ink / term.height > (layout === "b" ? 0.5 : 0.3), `only ${Math.round((ink / term.height) * 100)}% of the agent's window has anything in it`);
      // The bar is over the thing being built, never over the agent's prompt.
      const [bar, prompt] = [await t.box("[data-scene] .clipframes-web .bar"), await t.box("[data-agent-prompt]")];
      if (layout === "b") assert.equal(overlap(bar, prompt), false, "the bar is over the agent's prompt");
    }, layout ? `?scene=${layout}` : "");
  }
  for (const which of ORDER) {
    await session(`desktop, scene ${layout || "as built"}: the ${which} example plays to the end`, desktop, async (t) => {
      if (which !== "web") await t.page.locator(`[data-case="${which}"]`).click();
      await t.played(which);
      await t.ended(which);
      // The bar is back up for the visitor, and the thing being built is in front again.
      await t.bar().waitFor();
      await t.page.waitForFunction(() => document.querySelector("[data-scene] [role=group]").dataset.front === "app");
    }, layout ? `?scene=${layout}` : "");
  }
}

await session("desktop: an example leaves the clipboard alone, and nothing moves out of place", desktop, async (t) => {
  await t.page.evaluate(() => {
    window.shift = 0;
    new PerformanceObserver((list) => { for (const entry of list.getEntries()) if (!entry.hadRecentInput) window.shift += entry.value; }).observe({ type: "layout-shift", buffered: true });
  });
  await t.page.evaluate(() => navigator.clipboard.writeText("the visitor's own text"));
  await t.played("web");
  assert.equal(await t.clipboard(), "the visitor's own text");
  assert.ok((await t.page.evaluate(() => window.shift)) < 0.01, `layout shift ${await t.page.evaluate(() => window.shift)}`);
});

await session("desktop: left alone, the scene shows all four kinds of work in turn", desktop, async (t) => {
  for (const which of ORDER) {
    await t.played(which);
    await t.ended(which);
  }
});

await session("desktop: a visitor's pick in the desktop app is pasted, answered and carried out", desktop, async (t) => {
  await t.page.locator('[data-case="desktop"]').click();
  await t.played("desktop");
  const check = await t.at("#SoundCheck", 0.6);
  await t.page.mouse.move(check.x - 60, check.y + 50);
  await t.page.mouse.move(check.x, check.y, { steps: 8 });
  assert.equal(await t.page.locator("[data-scene] .highlight .label").innerText(), 'CheckBox "Play a sound when the time is up"');
  await t.pick("#ResetButton", "make this red");
  await t.answers(2);
  const said = await t.said();
  const text = '[Button "Reset" (id=ResetButton) in Focus: make this red]';
  assert.deepEqual(said.slice(3), [["you", "›", text], ["says", "•", 'Button "Reset" is red now.']]);
  assert.equal(await t.page.locator("[data-scene] #ResetButton").evaluate((el) => getComputedStyle(el).backgroundColor), "rgb(214, 51, 58)");
  assert.equal(await t.clipboard(), text);
  // As after a real paste: the round is over and the bar is ready for the next one.
  await t.bar().getByText("Click anything").waitFor();
  assert.equal(await t.page.locator("[data-scene] .clipframes-web .mark").count(), 0);
  // Having chosen, the visitor is not taken elsewhere.
  await t.page.waitForTimeout(6500);
  assert.equal(await t.page.locator("[data-scene]").getAttribute("data-scene"), "desktop");
});

await session("desktop: the agent only claims what was done: rename, hide, bigger, and a plain word for the rest", desktop, async (t) => {
  await t.played("web");
  await t.pick("#export", "rename to Download");
  await t.answers(2);
  assert.equal(await t.page.locator("[data-scene] #export").innerText(), "Download");
  assert.equal((await t.said()).at(-1)[2], 'Button "Export" now says "Download".');
  await t.pick("#paid-total strong", "bigger");
  await t.answers(3);
  assert.equal((await t.said()).at(-1)[2], 'Text "$48,210" is bigger now.');
  assert.equal(await t.page.locator("[data-scene] #paid-total strong").evaluate((el) => getComputedStyle(el).fontSize), "30px");
  await t.pick("#invoice-search", "hide this");
  await t.answers(4);
  assert.equal(await t.page.locator("[data-scene] #invoice-search").evaluate((el) => getComputedStyle(el).visibility), "hidden");
  await t.pick("#outstanding-total strong", "line this up with the heading");
  await t.answers(5);
  assert.equal((await t.said()).at(-1)[2], `Got Text "$12,940". ${ONLY}`);
  // Each round was the visitor's own: one thing, pasted once.
  assert.equal((await t.said()).filter(([who]) => who === "you").length, 5);
});

await session("desktop: stepping in while the example plays stops it, and the first pick is the visitor's alone", desktop, async (t) => {
  await t.page.waitForFunction(() => document.querySelector("[data-scene] .clipframes-web #comment")?.value.length > 3, null, { timeout: 12000 });
  await t.pick("#export", "make this green");
  await t.answers(1);
  assert.deepEqual(await t.said(), [["you", "❯", '[Button "Export" (#export .btn), under heading "Invoices" in Google Chrome "Invoices": make this green]'], ["says", "●", 'Button "Export" is green now.']]);
  assert.equal(await t.page.locator("[data-scene] #export").evaluate((el) => getComputedStyle(el).backgroundColor), "rgb(31, 157, 85)");
});

await session("desktop: an area in the game is a screenshot, and the agent says only that it got it", desktop, async (t) => {
  await t.page.locator('[data-case="game"]').click();
  await t.played("game");
  await t.bar().getByRole("tab", { name: "Screenshot an area" }).click();
  const from = await t.at("#view", 0.4, 0.5);
  await t.page.mouse.move(from.x, from.y);
  await t.page.mouse.down();
  await t.page.mouse.move(from.x + 160, from.y + 110, { steps: 8 });
  await t.page.mouse.up();
  await t.note().waitFor();
  await t.page.keyboard.type("make the ledge wider");
  await t.page.keyboard.press("Enter");
  await t.answers(2);
  assert.deepEqual((await t.said()).slice(3), [["you", "›", "[Screenshot (1.png) in Hopper: make the ledge wider]"], ["says", "•", `Got the screenshot. ${ONLY}`]]);
});

await session("desktop: the real recording is under the desktop app only, and is not fetched until asked for", desktop, async (t) => {
  assert.equal(await t.page.locator("[data-proof]").count(), 0);
  await t.page.locator('[data-case="desktop"]').click();
  await t.page.locator("[data-proof]").waitFor();
  assert.equal(await t.page.locator("[data-scene] video").count(), 0);
  await t.page.locator("[data-proof]").click();
  await t.page.waitForFunction(() => document.querySelector("dialog[open] video")?.currentTime > 0.3, null, { timeout: 15000 });
  assert.equal(await t.page.locator("dialog video").getAttribute("src"), "/recordings/native.mp4");
  await t.page.locator("dialog").getByRole("button", { name: "Close" }).click();
  assert.equal(await t.page.locator("dialog[open]").count(), 0);
  await t.page.locator('[data-case="motion"]').click();
  assert.equal(await t.page.locator("[data-proof]").count(), 0);
});

await session("desktop: the nav goes to Compare, which shows one request three ways and says where it read about Agentation", desktop, async (t) => {
  await t.page.locator("header nav").getByRole("link", { name: "Compare" }).click();
  await t.page.waitForURL("**/compare");
  assert.equal(await t.page.locator("main h1").innerText(), "One request, three ways.");
  assert.equal(await t.page.locator("main figure").count(), 3);
  const cards = await t.page.locator("main figure").allInnerTexts();
  assert.match(cards[0], /\[Image #1\] make the second paid badge grey/);
  assert.match(cards[1], /\*\*Source:\*\* src\/components\/InvoiceTable\.tsx:31:9/);
  assert.match(cards[2], /2nd of 4 on the page/);
  assert.equal(await t.page.locator('main a[href="https://www.agentation.com/output"]').count(), 1);
  assert.equal(/[–—]/.test(await t.page.locator("main").innerText()), false, "a dash is in the page");
  assert.equal(await t.page.locator('header nav a[aria-current="page"]').first().innerText(), "Compare");
  // The first screen holds the three, whole.
  const last = await t.box("main figure:nth-of-type(3)");
  assert.ok(last.y + last.height <= 900, `the three ways end at ${last.y + last.height}`);
});

await session("desktop: with reduced motion the scene goes straight to its result and stays", { ...desktop, reducedMotion: "reduce" }, async (t) => {
  await t.played("web");
  await t.ended("web");
  await t.page.locator('[data-case="motion"]').click();
  await t.played("motion");
  await t.ended("motion");
  await t.page.waitForTimeout(6500);
  assert.equal(await t.page.locator("[data-scene]").getAttribute("data-scene"), "motion");
});

await session("phone: the first screen is the headline, one line, the scene and the kinds of work", phone, async (t) => {
  const under = await t.box("[data-scene] figcaption");
  assert.ok(under.y + under.height <= 844, `the kinds of work end at ${under.y + under.height}`);
  assert.equal(await t.page.evaluate(() => [...document.querySelectorAll('a[href^="/download"]')].filter((a) => a.getBoundingClientRect().width > 0).length), 0, "a phone is offered a download");
});

for (const which of ORDER) {
  await session(`phone: the ${which} example plays to the end, with the agent as a sheet that comes up and goes down`, phone, async (t) => {
    if (which !== "web") await t.page.locator(`[data-case="${which}"]`).tap();
    // While the comment is being written the bar is not under the comment box.
    if (which === "web" || which === "desktop") {
      await t.note().waitFor({ timeout: 12000 });
      const [note, bar] = [await t.box("[data-scene] .clipframes-web .note"), await t.box("[data-scene] .clipframes-web .bar")];
      assert.equal(overlap(note, bar), false, "the comment box is over the bar");
    }
    await t.page.waitForFunction(() => document.querySelector("[data-scene] [role=group]").dataset.front === "agent", null, { timeout: 15000, polling: 100 });
    await t.played(which);
    await t.ended(which);
    await t.page.waitForFunction(() => document.querySelector("[data-scene] [role=group]").dataset.front === "app");
    // Down, the sheet leaves the thing and the bar clear.
    await t.page.waitForTimeout(600);
    const [term, bar] = [await t.box("[data-scene] [data-agent]"), await t.box("[data-scene] .clipframes-web .bar")];
    assert.equal(overlap(term, bar), false, "the agent's sheet is over the bar");
  });
}

await session("phone: a tap in the desktop app picks, and the agent answers", phone, async (t) => {
  await t.page.locator('[data-case="desktop"]').tap();
  await t.played("desktop");
  await t.page.waitForFunction(() => document.querySelector("[data-scene] [role=group]").dataset.front === "app");
  await t.page.waitForTimeout(600);
  const target = await t.at("#ResetButton");
  await t.page.touchscreen.tap(target.x, target.y);
  await t.note().waitFor();
  await t.note().fill("smaller");
  await t.page.locator("[data-scene] .note button.pill").tap();
  await t.answers(2);
  assert.deepEqual((await t.said()).slice(3), [["you", "›", '[Button "Reset" (id=ResetButton) in Focus: smaller]'], ["says", "•", 'Button "Reset" is smaller now.']]);
});

await browser.close();
console.log(failed ? `${failed} failed` : "all passed");
process.exit(failed ? 1 : 0);
