// Drives every interactive piece of the home page the way a visitor would, with a mouse at
// 1440 px and a finger at 390 px, and checks what each one does: the text each demo copies and
// shows in its prompt, what is in the first screen, the hero's three layouts, the recordings,
// and the downloads. Headless; nothing appears on screen.
//
//   npm run build && npx next start -p 5231 &     (or npm run dev)
//   node scripts/check-demos.mjs [http://localhost:5231] [folder for pictures]
//
// Needs Google Chrome.
import assert from "node:assert/strict";
import { mkdirSync } from "node:fs";
import { chromium } from "playwright-core";

const url = process.argv[2] ?? "http://localhost:5231";
const shots = process.argv[3];
if (shots) mkdirSync(shots, { recursive: true });
const IDS = ["hero", "picks", "area", "clip", "tab"];
const WHERE = 'in Google Chrome "Invoices"';
const UNDER = ', under heading "Invoices"';

const browser = await chromium.launch({ channel: "chrome", headless: true });
let failed = 0;

const SITE = "https://clipframes.app";
const HERO = '[Button "New invoice" (#new-invoice .btn.btn-primary), under heading "Invoices" in Google Chrome "Invoices": make this green]';

async function session(name, options, body, query = "") {
  const context = await browser.newContext({ ...options, permissions: ["clipboard-read", "clipboard-write"] });
  const page = await context.newPage();
  const errors = [];
  page.on("console", (message) => message.type() === "error" && errors.push(message.text()));
  page.on("pageerror", (error) => errors.push(String(error)));
  // Nothing may fail to load, and nothing may be asked of another host.
  page.on("requestfailed", (request) => errors.push(`failed: ${request.url()}`));
  page.on("response", (response) => response.status() >= 400 && errors.push(`${response.status()}: ${response.url()}`));
  await context.route((address) => address.origin !== new URL(url).origin, (route) => route.abort());
  const t = {
    page,
    stage: (id) => page.locator(`[data-demo="${id}"]`),
    /// The middle of something in a demo's Northwind page, on screen.
    async at(id, selector, fx = 0.5, fy = 0.5) {
      const box = await page.locator(`[data-demo="${id}"] .northwind ${selector}`).boundingBox();
      return { x: box.x + box.width * fx, y: box.y + box.height * fy };
    },
    /// Brings a demo to the middle of the screen and waits for its example to finish.
    async visit(id) {
      await page.evaluate((id) => document.querySelector(`[data-demo="${id}"] [role=group]`).scrollIntoView({ block: "center" }), id);
      await page.waitForTimeout(700);
      await page.waitForFunction((id) => document.querySelector(`[data-demo="${id}"] figcaption button[data-off]`) === null, id, { timeout: 30000 });
      await page.waitForTimeout(300);
    },
    /// What a demo's prompt shows, without the prompt's own mark.
    copied: async (which) => (await page.locator(`[data-copied="${which}"]`).innerText()).replace(/^> /, ""),
    /// "empty" (the faint example), "example" or "own".
    state: (which) => page.locator(`[data-copied="${which}"]`).getAttribute("data-state"),
    box: (selector) => page.locator(selector).first().boundingBox(),
    clipboard: () => page.evaluate(() => navigator.clipboard.readText()),
    bar: (id) => page.locator(`[data-demo="${id}"] .clipframes-web:not([data-still]) .bar`),
    note: (id) => page.locator(`[data-demo="${id}"] .clipframes-web:not([data-still]) #comment`),
  };
  try {
    await page.goto(url + query, { waitUntil: "networkidle" });
    await body(t);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false, "the page scrolls sideways");
    assert.deepEqual(errors, [], "the console has errors");
    console.log(`ok    ${name}`);
  } catch (error) {
    failed += 1;
    console.log(`FAIL  ${name}\n      ${String(error.message ?? error).split("\n").join("\n      ")}`);
    if (shots) await page.screenshot({ path: `${shots}/fail-${name.replace(/\W+/g, "-")}.png` });
  }
  await context.close();
}

/// One full-page picture, after every example has played and every part has been on screen.
async function picture(t, file) {
  for (const id of IDS) await t.visit(id);
  await t.page.evaluate(async () => {
    for (let y = 0; y < document.body.scrollHeight; y += 500) {
      scrollTo(0, y);
      await new Promise((r) => setTimeout(r, 60));
    }
    scrollTo(0, 0);
  });
  await t.page.waitForTimeout(700);
  await t.page.screenshot({ path: file, fullPage: true });
}

const desktop = { viewport: { width: 1440, height: 900 } };
const phone = { viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 };

/// The agent's text is in the hero's prompt within ten seconds of arriving, with nothing done.
const arrives = (t) => t.page.waitForFunction(() => document.querySelector('[data-copied="hero"]').dataset.state === "example" && document.querySelector('[data-copied="hero"]').innerText.includes("make this green"), null, { timeout: 10000 });

for (const [width, height] of [[1440, 900], [1280, 720]]) {
  for (const hero of ["", "a", "b", "c"]) {
    await session(`desktop ${width}×${height}, hero ${hero || "as built"}: the demo, what to do with it and the agent's text are in the first screen`, { viewport: { width, height } }, async (t) => {
      assert.equal(await t.page.evaluate(() => document.documentElement.dataset.hero), hero || "b");
      const stage = await t.box('[data-demo="hero"] [role=group]');
      const hint = await t.box('[data-demo="hero"] [data-hint]');
      const text = await t.box('[data-copied="hero"]');
      assert.ok(hint.y + hint.height <= height, "the sentence that says what to do is below the first screen");
      // The whole demo at 900 px; at 720 px at least the part the example uses, down to under the comment box.
      assert.ok(stage.y + (height >= 900 ? stage.height : 270) <= height, `the demo ends at ${stage.y + stage.height}`);
      // The prompt's first two lines, which is all one pick takes.
      assert.ok(text.y + 62 <= height, `the agent's text starts at ${text.y}`);
      assert.equal(await t.state("hero"), "empty");
      await arrives(t);
      assert.equal(await t.copied("hero"), HERO);
      assert.equal(await t.page.evaluate(() => scrollY), 0);
      // A download is in the first screen too.
      const get = await t.box('main a[href="/download/mac"]');
      assert.ok(get.y + get.height <= height, "the download is below the first screen");
    }, hero ? `?hero=${hero}` : "");
  }
}

await session("desktop: the hero's three layouts are three layouts", desktop, async (t) => {
  const places = {};
  for (const hero of ["a", "b", "c"]) {
    await t.page.goto(`${url}?hero=${hero}`, { waitUntil: "networkidle" });
    const [h1, stage, prompt] = [await t.box("main h1"), await t.box('[data-demo="hero"] [role=group]'), await t.box('[data-prompt="hero"]')];
    places[hero] = { beside: stage.x > h1.x + 300 && stage.y < h1.y + h1.height, promptBeside: prompt.x >= stage.x + stage.width, first: stage.y < h1.y };
  }
  assert.deepEqual(places, { a: { beside: true, promptBeside: false, first: false }, b: { beside: false, promptBeside: true, first: false }, c: { beside: false, promptBeside: true, first: true } });
  // A choice that is not one of the three is no choice.
  await t.page.goto(`${url}?hero=zzz`, { waitUntil: "networkidle" });
  assert.equal(await t.page.evaluate(() => document.documentElement.dataset.hero), "b");
});

await session("desktop: nothing on the page moves out of place while it loads and plays", desktop, async (t) => {
  await t.page.goto(url, { waitUntil: "domcontentloaded" });
  await t.page.evaluate(() => {
    window.shift = 0;
    new PerformanceObserver((list) => { for (const entry of list.getEntries()) if (!entry.hadRecentInput) window.shift += entry.value; }).observe({ type: "layout-shift", buffered: true });
  });
  await arrives(t);
  await t.page.waitForTimeout(500);
  assert.ok((await t.page.evaluate(() => window.shift)) < 0.01, `layout shift ${await t.page.evaluate(() => window.shift)}`);
});

await session("desktop: the examples play by themselves and leave the clipboard alone", desktop, async (t) => {
  await t.page.evaluate(() => navigator.clipboard.writeText("the visitor's own text"));
  await t.visit("hero");
  assert.equal(await t.note("hero").inputValue(), "make this green");
  assert.match(await t.bar("hero").innerText(), /1\s*copied/);
  assert.equal(await t.copied("hero"), HERO);
  assert.equal(await t.state("hero"), "example");
  await t.visit("picks");
  assert.equal(await t.copied("picks"), `[Clipframes: 3 things ${WHERE}]\n1. Button "New invoice" (#new-invoice .btn.btn-primary)${UNDER}: make this green\n2. Text "$3,120" (#overdue-total)${UNDER}: too alarming, use the normal text colour\n3. Text "Paid" (#invoice-table .badge.paid), 2nd of 4 on the page${UNDER}: make this one grey`);
  assert.equal(await t.stage("picks").locator(".clipframes-web:not([data-still]) .label").innerText(), 'Group "Outstanding $12,940"');
  await t.visit("area");
  assert.equal(await t.note("area").inputValue(), "put more space between these");
  assert.equal(await t.stage("area").locator(".mark.area").count(), 1);
  assert.equal(await t.copied("area"), `[Screenshot (1.png) ${WHERE}: put more space between these]`);
  await t.visit("clip");
  assert.equal(await t.note("clip").inputValue(), "the menu opens far from the button");
  assert.match(await t.stage("clip").locator(".clipframes-web:not([data-still]) .note .what").innerText(), /^Screen clip, [345] s, \d+ frames$/);
  assert.equal(await t.stage("clip").locator("#export-menu").count(), 1);
  assert.match(await t.copied("clip"), /^\[Screen clip, [345] s, \d+ frames \(1\/\) in Google Chrome "Invoices": the menu opens far from the button\]$/);
  await t.visit("tab");
  assert.equal(await t.stage("tab").locator(".tabmark").count(), 1);
  // The demos that are not live keep a still copy of what they showed.
  assert.equal(await t.stage("hero").locator("[data-still] .note").count(), 1);
  assert.equal(await t.clipboard(), "the visitor's own text");
  if (shots) await picture(t, `${shots}/desktop.png`);
});

await session("desktop: point at an element, click it, and write a comment", desktop, async (t) => {
  await t.visit("hero");
  const target = await t.at("hero", "#export");
  await t.page.mouse.move(target.x - 60, target.y + 40);
  await t.page.mouse.move(target.x, target.y, { steps: 8 });
  assert.equal(await t.stage("hero").locator(".highlight .label").innerText(), 'Button "Export"');
  await t.page.mouse.click(target.x, target.y);
  await t.note("hero").waitFor();
  await t.page.keyboard.type("make this smaller");
  await t.page.keyboard.press("Enter");
  const text = `[Clipframes: 2 things ${WHERE}]\n1. Button "New invoice" (#new-invoice .btn.btn-primary)${UNDER}: make this green\n2. Button "Export" (#export .btn)${UNDER}: make this smaller`;
  await t.page.waitForFunction((want) => document.querySelector('[data-copied="hero"]').innerText.includes(want), "make this smaller");
  assert.equal(await t.copied("hero"), text);
  assert.equal(await t.state("hero"), "own");
  assert.equal(await t.clipboard(), text);
  assert.match(await t.page.locator('[data-prompt="hero"] figcaption').innerText(), /on your own clipboard now/);
  // The prompt is still in the first screen with two picks in it.
  const prompt = await t.box('[data-copied="hero"]');
  assert.ok(prompt.y + prompt.height <= 900 + (await t.page.evaluate(() => scrollY)), "the visitor's text runs below the first screen");
  assert.match(await t.bar("hero").innerText(), /2\s*copied/);
  // History lists the round.
  assert.equal(await t.page.locator(".history li").count(), 5);
  assert.equal(await t.page.locator(".history li strong").first().innerText(), 'Button "New invoice" and 1 more');
});

await session("desktop: several picks, and the text follows along", desktop, async (t) => {
  await t.visit("picks");
  const target = await t.at("picks", "#paid-total", 0.9, 0.2);
  await t.page.mouse.move(target.x - 30, target.y - 30);
  await t.page.mouse.move(target.x, target.y, { steps: 6 });
  await t.page.mouse.click(target.x, target.y);
  await t.note("picks").waitFor();
  await t.page.keyboard.type("show last month too");
  await t.stage("picks").locator(".note button.pill").click();
  await t.page.waitForFunction(() => document.querySelector('[data-copied="picks"]').innerText.includes("4 things"));
  const lines = (await t.copied("picks")).split("\n");
  assert.equal(lines.length, 5);
  assert.equal(lines[4], `4. Group "Paid this month $48,210" (#paid-total .stat)${UNDER}: show last month too`);
  assert.equal(await t.clipboard(), lines.join("\n"));
  assert.equal(await t.stage("picks").locator(".mark").count(), 4);
  // Remove takes a pick out again.
  const second = await t.at("picks", "#export");
  await t.page.mouse.click(second.x, second.y);
  await t.note("picks").waitFor();
  await t.stage("picks").locator(".note button.quiet").click();
  await t.page.waitForTimeout(200);
  assert.equal((await t.copied("picks")).split("\n").length, 5);
});

await session("desktop: drag an area", desktop, async (t) => {
  await t.visit("area");
  const from = await t.at("area", "#invoice-table", 0.05, 0.6);
  const to = await t.at("area", "#invoice-table", 0.7, 0.95);
  await t.page.mouse.move(from.x, from.y, { steps: 4 });
  await t.page.mouse.down();
  await t.page.mouse.move((from.x + to.x) / 2, (from.y + to.y) / 2, { steps: 6 });
  assert.match(await t.stage("area").locator(".drag .size").innerText(), /^\d+ × \d+$/);
  await t.page.mouse.move(to.x, to.y, { steps: 6 });
  await t.page.mouse.up();
  await t.note("area").waitFor();
  await t.page.keyboard.type("the table is cramped");
  await t.page.keyboard.press("Enter");
  await t.page.waitForFunction(() => document.querySelector('[data-copied="area"]').innerText.includes("cramped"));
  assert.equal(await t.copied("area"), `[Clipframes: 2 things ${WHERE}]\n1. Screenshot (1.png): put more space between these\n2. Screenshot (2.png): the table is cramped`);
});

await session("desktop: record a clip, use the page, and stop", desktop, async (t) => {
  await t.visit("clip");
  await t.stage("clip").getByRole("button", { name: "Play the example again" }).click();
  const middle = await t.at("clip", "#invoice-table", 0.5, 0.5);
  await t.page.mouse.move(middle.x + 40, middle.y + 30);
  await t.page.mouse.move(middle.x, middle.y, { steps: 5 });
  await t.page.waitForTimeout(300);
  assert.equal(await t.stage("clip").locator("#export-menu").count(), 0);
  const from = await t.at("clip", "#invoice-table", 0.02, 0.1);
  const to = await t.at("clip", "#invoice-table", 0.9, 0.6);
  await t.page.mouse.move(from.x, from.y);
  await t.page.mouse.down();
  await t.page.mouse.move(to.x, to.y, { steps: 8 });
  await t.page.mouse.up();
  await t.bar("clip").getByText("Recording").waitFor();
  // While it records, the page under it is used as usual.
  const button = await t.at("clip", "#export");
  await t.page.mouse.click(button.x, button.y);
  assert.equal(await t.stage("clip").locator("#export-menu").count(), 1);
  await t.page.waitForTimeout(1300);
  await t.bar("clip").getByRole("button", { name: "Stop" }).click();
  await t.note("clip").waitFor();
  await t.page.keyboard.type("nothing happens when I click a row");
  await t.page.keyboard.press("Enter");
  await t.page.waitForFunction(() => document.querySelector('[data-copied="clip"]').innerText.includes("nothing happens"));
  assert.match(await t.copied("clip"), /^\[Screen clip, [12] s, [48] frames \(1\/\) in Google Chrome "Invoices": nothing happens when I click a row\]$/);
});

await session("desktop: History copies an earlier capture again", desktop, async (t) => {
  await t.page.locator("#history").scrollIntoViewIfNeeded();
  assert.equal(await t.page.locator(".history li").count(), 4);
  await t.page.locator(".history li").nth(3).getByRole("button", { name: "Copy" }).click();
  const text = `[Screenshot (1.png) ${WHERE}: the table is cramped]`;
  await t.page.waitForFunction((text) => navigator.clipboard.readText().then((now) => now === text), text);
  assert.equal(await t.page.locator(".history li").nth(3).getByRole("button").first().innerText(), "Copied");
  // The page is not marked as one of the app's windows, which would stop the demos picking.
  assert.equal(await t.page.evaluate(() => document.documentElement.classList.contains("window")), false);
});

await session("desktop: the tab opens the bar, and the pin turns the tab off", desktop, async (t) => {
  await t.visit("tab");
  assert.equal(await t.bar("tab").count(), 0);
  await t.stage("tab").locator(".tabmark").click();
  await t.bar("tab").waitFor();
  assert.equal(await t.stage("tab").locator(".tabmark").count(), 0);
  await t.bar("tab").getByRole("button", { name: "Close" }).click();
  await t.stage("tab").locator(".tabmark").waitFor();
  await t.stage("tab").locator(".tabmark").click();
  await t.bar("tab").getByRole("button", { name: "Appear here by itself" }).click();
  await t.bar("tab").getByRole("button", { name: "Close" }).click();
  await t.stage("tab").getByRole("button", { name: "Turn it back on" }).waitFor();
  assert.equal(await t.stage("tab").locator(".tabmark").count(), 0);
  await t.stage("tab").getByRole("button", { name: "Turn it back on" }).click();
  await t.stage("tab").locator(".tabmark").waitFor();
});

await session("desktop: the clock in the bar goes to History, and Esc closes the bar", desktop, async (t) => {
  await t.visit("hero");
  const before = await t.page.evaluate(() => scrollY);
  await t.bar("hero").getByRole("button", { name: "History" }).click();
  await t.page.waitForFunction((y) => scrollY > y + 1000, before);
  await t.visit("picks");
  const over = await t.at("picks", "#invoice-table", 0.5, 0.2);
  await t.page.mouse.move(over.x + 40, over.y - 20);
  await t.page.mouse.move(over.x, over.y, { steps: 5 });
  await t.page.keyboard.press("Escape");
  await t.stage("picks").locator(".tabmark").waitFor();
});

await session("desktop: with reduced motion each demo goes straight to its result", { ...desktop, reducedMotion: "reduce" }, async (t) => {
  await t.page.waitForTimeout(800);
  assert.equal(await t.note("hero").inputValue(), "make this green");
  await t.visit("picks");
  assert.match(await t.copied("picks"), /^\[Clipframes: 3 things/);
  await t.visit("area");
  assert.equal(await t.note("area").inputValue(), "put more space between these");
  await t.visit("clip");
  assert.equal(await t.note("clip").inputValue(), "the menu opens far from the button");
});

await session("phone: the examples play, and nothing is wider than the screen", phone, async (t) => {
  await t.visit("hero");
  assert.equal(await t.note("hero").inputValue(), "make this green");
  await t.visit("picks");
  assert.match(await t.copied("picks"), /^\[Clipframes: 3 things/);
  if (shots) await picture(t, `${shots}/phone.png`);
  assert.equal(await t.page.evaluate(() => getComputedStyle(document.querySelector("[data-hint] span")).display), "none", "a phone gets the hint for a finger");
});

await session("phone: a tap picks, and a swipe scrolls without picking", phone, async (t) => {
  await t.visit("picks");
  const target = await t.at("picks", "#export");
  await t.page.touchscreen.tap(target.x, target.y);
  await t.note("picks").waitFor();
  await t.note("picks").fill("move this into a menu");
  await t.stage("picks").locator(".note button.pill").tap();
  await t.page.waitForFunction(() => document.querySelector('[data-copied="picks"]').innerText.includes("4 things"));
  assert.equal((await t.copied("picks")).split("\n")[4], `4. Button "Export" (#export .btn)${UNDER}: move this into a menu`);
  // A swipe up over the stage scrolls the page and picks nothing.
  const client = await t.page.context().newCDPSession(t.page);
  const start = await t.at("picks", "#invoice-table", 0.5, 0.5);
  const y0 = await t.page.evaluate(() => scrollY);
  await client.send("Input.synthesizeScrollGesture", { x: start.x, y: start.y, yDistance: -260, speed: 900, gestureSourceType: "touch" });
  await t.page.waitForTimeout(300);
  assert.ok((await t.page.evaluate(() => scrollY)) > y0 + 150, "the swipe did not scroll the page");
  assert.match(await t.copied("picks"), /4 things/);
});

await session("phone: a drag that starts sideways draws an area", phone, async (t) => {
  await t.visit("area");
  const client = await t.page.context().newCDPSession(t.page);
  const from = await t.at("area", "#invoice-table", 0.1, 0.7);
  const to = await t.at("area", "#invoice-table", 0.85, 0.86);
  const touch = (type, x, y) => client.send("Input.dispatchTouchEvent", { type, touchPoints: type === "touchEnd" ? [] : [{ x: Math.round(x), y: Math.round(y) }] });
  await touch("touchStart", from.x, from.y);
  for (let i = 1; i <= 6; i++) await touch("touchMove", from.x + i * 12, from.y + i);
  for (let i = 1; i <= 8; i++) await touch("touchMove", from.x + 72 + ((to.x - from.x - 72) * i) / 8, from.y + 6 + ((to.y - from.y - 6) * i) / 8);
  await touch("touchEnd");
  await t.page.waitForFunction(() => document.querySelectorAll('[data-demo="area"] .clipframes-web:not([data-still]) .mark.area').length === 2, null, { timeout: 5000 });
  await t.note("area").fill("too tight");
  await t.stage("area").locator(".note button.pill").tap();
  await t.page.waitForFunction(() => document.querySelector('[data-copied="area"]').innerText.includes("too tight"));
  assert.match(await t.copied("area"), /2\. Screenshot \(2\.png\): too tight$/);
});

await session("desktop: what a visitor asks before downloading is answered on the page, and the downloads are downloads", desktop, async (t) => {
  // The line by the first download goes to the answers.
  await t.page.getByRole("link", { name: "What it asks for and what it sends" }).click();
  await t.page.waitForFunction(() => scrollY > 3000);
  const facts = await t.page.locator("#get").innerText();
  for (const said of ["Clipframes is free.", "macOS 12 or later", "Windows 10 or 11", "about 5 MB for Mac and about 2 MB for Windows", "22 to 26 MB", "Accessibility", "Screen Recording", "Windows asks for none", "They stay on your computer", "Settings has a switch", "Claude Code, Codex"]) assert.ok(facts.includes(said), `not said: ${said}`);
  assert.equal(await t.page.locator('#get a[href="/privacy"]').count(), 1);
  const whole = await t.page.locator("main").first().innerText();
  for (const said of ["24 of 24", "18 of 24", "2nd of 4 on the page", "A screenshot", "Agentation", "A browser extension", "native apps"]) assert.ok(whole.includes(said), `not said: ${said}`);
  // No dash of either long kind in anything a visitor reads.
  assert.equal(/[\u2013\u2014]/.test(await t.page.locator("body").innerText()), false, "a dash is in the page");
  // Five downloads: the header's, and a pair at the top and at the end. The phone's button is not shown.
  const shown = await t.page.evaluate(() => [...document.querySelectorAll('a[href^="/download"]')].filter((a) => a.getBoundingClientRect().width > 0).map((a) => a.getAttribute("href")));
  assert.deepEqual(shown, ["/download", "/download/mac", "/download/windows", "/download/mac", "/download/windows"]);
  assert.equal(await t.page.evaluate(() => [...document.querySelectorAll("[data-copy-link]")].filter((b) => b.getBoundingClientRect().width > 0).length), 0);
  // The header's stays on screen at the bottom of the page.
  await t.page.evaluate(() => scrollTo(0, document.body.scrollHeight));
  await t.page.waitForTimeout(300);
  const top = await t.box('header a[href="/download"]');
  assert.ok(top.y >= 0 && top.y < 60, "the header's download is not on screen at the end of the page");
});

await session("desktop: the recording is fetched only when it comes near, plays on screen and can be paused", desktop, async (t) => {
  const video = t.page.locator('[data-recording="desktop"] video');
  assert.equal(await video.getAttribute("src"), null, "the recording was fetched with the page");
  const shape = await video.boundingBox();
  assert.ok(Math.abs(shape.width / shape.height - 1920 / 1044) < 0.01, "the recording's box has another shape than the recording");
  await video.scrollIntoViewIfNeeded();
  await t.page.waitForFunction(() => document.querySelector('[data-recording="desktop"] video').currentTime > 0.3, null, { timeout: 15000 });
  assert.equal(await video.getAttribute("src"), "/recordings/desktop.mp4");
  const after = await video.boundingBox();
  assert.equal(Math.round(after.height), Math.round(shape.height));
  await t.page.locator('[data-recording="desktop"]').getByRole("button", { name: "Pause" }).click();
  assert.equal(await video.evaluate((v) => v.paused), true);
  await t.page.locator('[data-recording="desktop"]').getByRole("button", { name: "Play" }).click();
  assert.equal(await video.evaluate((v) => v.paused), false);
  // The two close-ups are a phone's, and are not fetched here.
  assert.equal(await t.page.locator('[data-recording="comment"] video').getAttribute("src"), null);
});

await session("phone: the first screen has the agent's text and the demo, and the example fills the text in", phone, async (t) => {
  const text = await t.box('[data-copied="hero"]');
  const stage = await t.box('[data-demo="hero"] [role=group]');
  assert.ok(text.y + text.height <= 844, `the agent's text ends at ${text.y + text.height}`);
  assert.ok(stage.y + 250 <= 844, `the demo starts at ${stage.y}`);
  await arrives(t);
  assert.equal(await t.copied("hero"), HERO);
});

await session("phone: a tap in the hero picks, and the text is the visitor's", phone, async (t) => {
  await arrives(t);
  const target = await t.at("hero", "#export");
  await t.page.touchscreen.tap(target.x, target.y);
  await t.note("hero").waitFor();
  await t.note("hero").fill("smaller");
  await t.stage("hero").locator(".note button.pill").tap();
  await t.page.waitForFunction(() => document.querySelector('[data-copied="hero"]').innerText.includes("smaller"));
  assert.equal(await t.state("hero"), "own");
  assert.equal((await t.copied("hero")).split("\n")[2], `2. Button "Export" (#export .btn)${UNDER}: smaller`);
});

await session("phone: the downloads are a button that copies the link, and the recordings are the close-ups", phone, async (t) => {
  assert.equal(await t.page.evaluate(() => [...document.querySelectorAll('a[href^="/download"]')].filter((a) => a.getBoundingClientRect().width > 0).length), 0, "a phone is offered a download");
  const buttons = t.page.locator("main [data-copy-link]");
  assert.equal(await buttons.count(), 2);
  await buttons.first().scrollIntoViewIfNeeded();
  await buttons.first().tap();
  await t.page.waitForFunction(() => document.querySelector("main [data-copy-link]").innerText === "Link copied");
  assert.equal(await t.clipboard(), SITE);
  assert.match(await t.page.locator("main").first().innerText(), /runs on Mac and Windows\. Open the link on your computer/);
  await t.page.locator("header [data-copy-link]").tap();
  await t.page.waitForFunction(() => document.querySelector("header [data-copy-link]").innerText === "Link copied");
  // The close-ups play; the recording of the whole desktop, too small to read here, is not fetched.
  const close = t.page.locator('[data-recording="comment"] video');
  await close.scrollIntoViewIfNeeded();
  await t.page.waitForFunction(() => document.querySelector('[data-recording="comment"] video').currentTime > 0.3, null, { timeout: 15000 });
  assert.equal(await t.page.locator('[data-recording="desktop"] video').getAttribute("src"), null);
  await t.page.locator('[data-recording="comment"]').getByRole("button", { name: "Pause" }).tap();
  assert.equal(await close.evaluate((v) => v.paused), true);
});

await browser.close();
console.log(failed ? `${failed} failed` : "all passed");
process.exit(failed ? 1 : 0);
