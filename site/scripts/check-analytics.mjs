// Checks what the site sends for analytics, in a real browser. Headless.
//
//   node scripts/check-analytics.mjs http://localhost:5231 off   (a build without NEXT_PUBLIC_POSTHOG_KEY)
//   node scripts/check-analytics.mjs http://localhost:5231 on    (a build with one)
//
// Requests to PostHog are caught here and never leave the machine.
import assert from "node:assert/strict";
import { gunzipSync } from "node:zlib";
import { chromium } from "playwright-core";

const [url = "http://localhost:5231", mode = "off"] = process.argv.slice(2);
// The library sends nothing for a browser that says it is automated or headless, so this one does not say so.
const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--disable-blink-features=AutomationControlled"] });
const context = await browser.newContext({ userAgent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36", viewport: { width: 1280, height: 900 }, permissions: ["clipboard-read", "clipboard-write"] });
const page = await context.newPage();
const outside = [];
const scripts = [];
const events = [];
const errors = [];
page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
page.on("request", (request) => {
  const address = new URL(request.url());
  if (address.origin !== new URL(url).origin) outside.push(request.url());
  else if (address.pathname.endsWith(".js")) scripts.push(address.pathname);
});
await context.route("https://eu.i.posthog.com/**", async (route) => {
  const request = route.request();
  let body = request.postDataBuffer() ?? Buffer.alloc(0);
  if (body[0] === 0x1f && body[1] === 0x8b) body = gunzipSync(body);
  let text = body.toString("utf8");
  if (text.startsWith("data=")) text = Buffer.from(decodeURIComponent(text.slice(5)), "base64").toString("utf8");
  try {
    const parsed = JSON.parse(text);
    events.push(...(Array.isArray(parsed) ? parsed : (parsed.batch ?? [parsed])));
  } catch {
    events.push({ event: "(unreadable)", properties: {}, raw: text.slice(0, 200) });
    console.log("unreadable:", request.method(), request.url(), request.headers()["content-type"], body.subarray(0, 12).toString("hex"));
  }
  await route.fulfill({ status: 200, contentType: "application/json", body: "{}" });
});

await page.goto(url, { waitUntil: "networkidle" });
// Let the hero's example put its text in the prompt, take the demo over, pick something, copy
// the text, play the recording, and press a download button at the top and at the end.
const stage = page.locator('[data-demo="hero"]');
await page.evaluate(() => document.querySelector('[data-demo="hero"] [role=group]').scrollIntoView({ block: "center" }));
await page.waitForFunction(() => document.querySelector('[data-copied="hero"]').dataset.state === "example", null, { timeout: 12000 });
await page.waitForTimeout(600);
const box = await stage.locator(".northwind #export").boundingBox();
await page.mouse.move(box.x - 40, box.y + 60);
await page.mouse.move(box.x + 20, box.y + 12, { steps: 6 });
await page.mouse.click(box.x + 20, box.y + 12);
await page.keyboard.type("a secret comment");
await page.keyboard.press("Enter");
await page.waitForFunction(() => document.querySelector('[data-copied="hero"]').dataset.state === "own");
await page.locator('[data-copied="hero"]').scrollIntoViewIfNeeded();
await page.locator('[data-copied="hero"]').click({ clickCount: 3 });
await page.keyboard.press("ControlOrMeta+C");
await page.locator('[data-recording="desktop"] video').scrollIntoViewIfNeeded();
await page.waitForFunction(() => document.querySelector('[data-recording="desktop"] video').currentTime > 0.2, null, { timeout: 15000 });
await page.evaluate(() => document.querySelectorAll('a[href^="/download"]').forEach((a) => a.addEventListener("click", (e) => e.preventDefault())));
await page.locator('main a[href="/download/windows"]').last().scrollIntoViewIfNeeded();
await page.locator('main a[href="/download/windows"]').last().click();
await page.evaluate(() => scrollTo(0, 0));
await page.locator('main a[href="/download/mac"]').first().click();
await page.locator('header a[href="/download"]').click();
// Events are sent a few at a time, every few seconds.
await page.waitForTimeout(4000);
await page.goto(`${url}/privacy`, { waitUntil: "networkidle" });
await page.waitForTimeout(4000);

const stored = await page.evaluate(() => ({ cookies: document.cookie, local: Object.keys(localStorage), session: Object.keys(sessionStorage) }));
assert.deepEqual([stored.cookies, stored.local, stored.session], ["", [], []], "something was kept in the browser");
assert.deepEqual(await context.cookies(), [], "a cookie was set");
assert.deepEqual(errors, [], "the console has errors");

if (mode === "off") {
  assert.deepEqual(outside, [], "a request left the site");
  assert.deepEqual(events, []);
  const body = await page.locator("main").innerText();
  assert.match(body, /does not count visits or clicks/);
  console.log(`ok    no key: no request outside the site, ${scripts.length} scripts loaded, nothing kept in the browser`);
} else {
  assert.ok(outside.every((address) => address.startsWith("https://eu.i.posthog.com/")), `a request went elsewhere: ${outside.join(", ")}`);
  const names = events.map((e) => e.event);
  for (const name of ["$pageview", "demo_started", "demo_finished", "reference_shown", "reference_copied", "recording_played", "download_clicked"]) assert.ok(names.includes(name), `${name} was not sent (got ${names.join(", ")})`);
  assert.equal(names.filter((n) => n === "$pageview").length, 2);
  const allowed = new Set(["token", "distinct_id", "$pathname", "$os", "$browser", "$device_type", "$lib", "$lib_version", "$geoip_disable", "$process_person_profile", "os", "place", "demo", "by"]);
  for (const e of events) {
    const extra = Object.keys(e.properties).filter((k) => !allowed.has(k));
    assert.deepEqual(extra, [], `${e.event} carries more than it should`);
    assert.equal(e.properties.$geoip_disable, true);
    assert.equal(e.properties.$process_person_profile, false);
    assert.ok(!JSON.stringify(e).includes("secret") && !JSON.stringify(e).includes("Export"), `${e.event} carries something from a demo`);
  }
  // Each download says which button it was: the pair at the end, the pair at the top, the header's.
  assert.deepEqual(events.filter((e) => e.event === "download_clicked").map((e) => `${e.properties.os} ${e.properties.place}`), ["windows end", "mac hero", "auto header"]);
  // The agent's text was on screen twice in the hero: the example's, then the visitor's own.
  assert.deepEqual(events.filter((e) => e.event === "reference_shown" && e.properties.demo === "hero").map((e) => e.properties.by), ["example", "visitor"]);
  assert.equal(events.find((e) => e.event === "recording_played").properties.place, "desktop");
  assert.deepEqual(events.find((e) => e.event === "demo_started").properties.demo, "hero");
  for (const name of ["download_clicked", "link_copied", "reference_shown", "recording_played"]) assert.ok((await page.locator("main").innerText()).includes(name), `the privacy page does not list ${name}`);
  console.log(`ok    with a key: ${names.join(", ")}; only the listed properties; nothing kept in the browser`);
  console.log(JSON.stringify(events.find((e) => e.event === "download_clicked").properties));
}
await browser.close();
