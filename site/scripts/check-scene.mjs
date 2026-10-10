// Checks the first screen's scene the way a visitor meets it, with a mouse at 1440 px and a
// finger at 390 px: what is in the first screen, that each kind of work plays its example to
// the end (the pick, the text in the agent's prompt, the agent's answer, the changed thing),
// that the scene moves on by itself and stops when the visitor steps in, and that a visitor's
// own pick lands in the agent's prompt in the right words. Headless; nothing appears on screen.
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

const WEB = '[Button "New invoice" (#new-invoice .btn.btn-primary), under heading "Invoices" in Google Chrome "Invoices": make this black]';
const DESKTOP = '[Button "Start" (id=StartButton) in Focus: make this green]';

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
    played: (which) => page.waitForFunction((which) => { const scene = document.querySelector("[data-scene]"); return scene.dataset.scene === which && scene.dataset.playing === undefined && document.querySelector("[data-scene] p[class*=says]") !== null; }, which, { timeout: 20000 }),
    said: () => page.locator("[data-scene] [aria-live] p").allInnerTexts(),
    typed: () => page.locator("[data-agent-prompt] span").innerText(),
    at: async (selector, fx = 0.5, fy = 0.5) => { const box = await page.locator(`[data-scene] ${selector}`).boundingBox(); return { x: box.x + box.width * fx, y: box.y + box.height * fy }; },
    note: () => page.locator("[data-scene] .clipframes-web:not([data-still]) #comment"),
    clipboard: () => page.evaluate(() => navigator.clipboard.readText()),
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

for (const [width, height] of [[1440, 900], [1280, 720]]) {
  await session(`desktop ${width}×${height}: the first screen is the headline, one line, one download and the scene`, { viewport: { width, height } }, async (t) => {
    const desk = await t.box("[data-scene] [role=group]");
    const cases = await t.box("[data-scene] figcaption");
    assert.ok(desk.y + desk.height <= height, `the scene ends at ${desk.y + desk.height}`);
    if (height >= 900) assert.ok(cases.y + cases.height <= height, `the kinds of work end at ${cases.y + cases.height}`);
    assert.equal(await t.page.locator("main h1:not(.northwind h1)").count(), 1);
    assert.deepEqual(await t.page.evaluate(() => [...document.querySelectorAll('a[href^="/download"]')].filter((a) => getComputedStyle(a).visibility !== "hidden" && a.getBoundingClientRect().width > 0).map((a) => a.textContent)), ["Download Clipframes"], "more than one download is showing");
    // The words in the first screen, outside the scene: the bar's, the headline, the line, the button.
    const words = await t.page.evaluate(() => [...document.querySelectorAll("header, main h1:not(.northwind h1), [data-walk=line], main a")].map((el) => el.innerText).join(" ").split(/\s+/).filter(Boolean).length);
    assert.ok(words < 45, `${words} words outside the scene`);
  });
}

await session("desktop: the web app's example plays to the end, and nothing moves out of place", desktop, async (t) => {
  await t.page.evaluate(() => {
    window.shift = 0;
    new PerformanceObserver((list) => { for (const entry of list.getEntries()) if (!entry.hadRecentInput) window.shift += entry.value; }).observe({ type: "layout-shift", buffered: true });
  });
  await t.page.evaluate(() => navigator.clipboard.writeText("the visitor's own text"));
  await t.played("web");
  assert.deepEqual(await t.said(), [`>\n${WEB}`, "●\nUpdate(src/components/InvoiceHeader.tsx)", "●\nThe New invoice button is black now."]);
  assert.equal(await t.page.locator("[data-scene] [data-agent]").getAttribute("data-agent"), "claude");
  assert.equal(await t.page.locator("[data-scene] #new-invoice").evaluate((el) => getComputedStyle(el).backgroundColor), "rgb(23, 24, 28)");
  // An example leaves the visitor's clipboard alone, and the bar is back up for them.
  assert.equal(await t.clipboard(), "the visitor's own text");
  await t.page.locator("[data-scene] .clipframes-web .bar").waitFor();
  assert.ok((await t.page.evaluate(() => window.shift)) < 0.01, `layout shift ${await t.page.evaluate(() => window.shift)}`);
});

await session("desktop: left alone, the scene goes on to the desktop app, with Codex, in a native window's words", desktop, async (t) => {
  await t.played("web");
  await t.played("desktop");
  assert.deepEqual(await t.said(), [`›\n${DESKTOP}`, "•\nEdited MainWindow.xaml", "•\nThe Start button is green now."]);
  assert.equal(await t.page.locator("[data-scene] [data-agent]").getAttribute("data-agent"), "codex");
  assert.equal(await t.page.locator("[data-scene] #StartButton").evaluate((el) => getComputedStyle(el).backgroundColor), "rgb(31, 157, 85)");
  assert.equal(await t.page.locator('[data-case="desktop"]').getAttribute("aria-selected"), "true");
  // It is a desktop window: no address, and its parts have no class names to copy.
  assert.equal(await t.page.locator("[data-scene] [data-kind=desktop]").innerText().then((text) => text.includes("localhost")), false);
});

await session("desktop: a visitor's own pick in the desktop app is in the agent's prompt and on their clipboard", desktop, async (t) => {
  await t.page.locator('[data-case="desktop"]').click();
  await t.played("desktop");
  const check = await t.at("#SoundCheck", 0.6);
  await t.page.mouse.move(check.x - 60, check.y + 50);
  await t.page.mouse.move(check.x, check.y, { steps: 8 });
  assert.equal(await t.page.locator("[data-scene] .highlight .label").innerText(), 'CheckBox "Play a sound when the time is up"');
  await t.page.mouse.click(check.x, check.y);
  await t.note().waitFor();
  assert.equal(await t.page.locator("[data-scene] .note .what").innerText(), 'CheckBox "Play a sound when the time is up"');
  await t.page.keyboard.type("off by default");
  await t.page.keyboard.press("Enter");
  const text = '[CheckBox "Play a sound when the time is up" (id=SoundCheck) in Focus: off by default]';
  await t.page.waitForFunction((text) => document.querySelector("[data-agent-prompt] span").innerText === text, text);
  assert.equal(await t.clipboard(), text);
  // Having chosen, the visitor is not taken elsewhere.
  await t.page.waitForTimeout(6500);
  assert.equal(await t.page.locator("[data-scene]").getAttribute("data-scene"), "desktop");
  assert.equal(await t.typed(), text);
});

await session("desktop: stepping in while the example plays stops it, and the first pick is the visitor's alone", desktop, async (t) => {
  await t.page.waitForFunction(() => document.querySelector("[data-scene] .clipframes-web #comment")?.value.length > 3, null, { timeout: 12000 });
  const target = await t.at("#export");
  await t.page.mouse.move(target.x - 80, target.y + 80);
  await t.page.mouse.move(target.x, target.y, { steps: 8 });
  await t.page.mouse.click(target.x, target.y);
  await t.note().waitFor();
  await t.page.keyboard.type("smaller");
  await t.page.keyboard.press("Enter");
  const text = '[Button "Export" (#export .btn), under heading "Invoices" in Google Chrome "Invoices": smaller]';
  await t.page.waitForFunction((text) => document.querySelector("[data-agent-prompt] span").innerText === text, text);
  assert.equal(await t.clipboard(), text);
  assert.equal(await t.page.locator("[data-scene] .clipframes-web .mark").count(), 1);
  assert.deepEqual(await t.said(), []);
});

await session("desktop: the kinds of work not built yet cannot be chosen, and the nav goes to Compare", desktop, async (t) => {
  assert.equal(await t.page.locator('[data-case="motion"]').isDisabled(), true);
  assert.equal(await t.page.locator('[data-case="game"]').isDisabled(), true);
  await t.page.locator("header nav").getByRole("link", { name: "Compare" }).click();
  await t.page.waitForURL("**/compare");
  assert.equal(await t.page.locator("main h1").innerText(), "Three ways to tell an agent which button.");
  assert.equal(await t.page.locator("main table tbody tr").count(), 5);
  assert.equal(/[–—]/.test((await t.page.locator("main").innerText()).replace(/–/g, "")), false);
  assert.equal(await t.page.locator('header nav a[aria-current="page"]').first().innerText(), "Compare");
});

await session("desktop: with reduced motion the scene goes straight to its result and stays", { ...desktop, reducedMotion: "reduce" }, async (t) => {
  await t.played("web");
  assert.equal((await t.said())[0], `>\n${WEB}`);
  await t.page.waitForTimeout(6500);
  assert.equal(await t.page.locator("[data-scene]").getAttribute("data-scene"), "web");
});

await session("phone: the first screen is the headline, one line, the scene and the kinds of work, and the example plays", phone, async (t) => {
  const cases = await t.box("[data-scene] figcaption");
  assert.ok(cases.y + cases.height <= 844, `the kinds of work end at ${cases.y + cases.height}`);
  assert.equal(await t.page.evaluate(() => [...document.querySelectorAll('a[href^="/download"]')].filter((a) => a.getBoundingClientRect().width > 0).length), 0, "a phone is offered a download");
  await t.played("web");
  assert.equal((await t.said())[0], `>\n${WEB}`);
});

await session("phone: the desktop app by a tap, and a tap in it picks", phone, async (t) => {
  await t.page.locator('[data-case="desktop"]').tap();
  await t.played("desktop");
  assert.equal((await t.said())[0], `›\n${DESKTOP}`);
  const target = await t.at("#ResetButton");
  await t.page.touchscreen.tap(target.x, target.y);
  await t.note().waitFor();
  await t.note().fill("ask first");
  await t.page.locator("[data-scene] .note button.pill").tap();
  const text = '[Button "Reset" (id=ResetButton) in Focus: ask first]';
  await t.page.waitForFunction((text) => document.querySelector("[data-agent-prompt] span").innerText === text, text);
});

await browser.close();
console.log(failed ? `${failed} failed` : "all passed");
process.exit(failed ? 1 : 0);
