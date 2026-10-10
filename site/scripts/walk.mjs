// Walks the home page as a first-time visitor would see it, at a laptop's size and a phone's,
// and writes down where everything is: one picture per screen, the whole page as one picture,
// a contact sheet of the screens, and a map of every heading, demo and download button with
// how far down it is in pixels and in screens. Headless; nothing appears on screen.
//
//   node scripts/walk.mjs [http://localhost:5231] [folder to write] [query, e.g. hero=b]
//
// Needs Google Chrome.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { chromium } from "playwright-core";

const url = process.argv[2] ?? "http://localhost:5231";
const out = path.resolve(process.argv[3] ?? "walk");
const query = process.argv[4] ? `?${process.argv[4]}` : "";
mkdirSync(out, { recursive: true });

const SIZES = [
  { name: "1440", options: { viewport: { width: 1440, height: 900 } } },
  { name: "390", options: { viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 } },
];

const browser = await chromium.launch({ channel: "chrome", headless: true });
const report = {};

for (const { name, options } of SIZES) {
  const context = await browser.newContext(options);
  const page = await context.newPage();
  await page.goto(url + query, { waitUntil: "networkidle" });
  await page.waitForTimeout(600);
  const { height } = options.viewport;

  // The first screen as it is the moment the page has loaded, before anything has played.
  await page.screenshot({ path: `${out}/${name}-first.jpg`, type: "jpeg", quality: 72 });

  const map = await page.evaluate((height) => {
    const y = (el) => Math.round(el.getBoundingClientRect().top + scrollY);
    const row = (kind, el, text) => {
      const r = el.getBoundingClientRect();
      return { kind, text: text.replace(/\s+/g, " ").trim().slice(0, 90), y: y(el), bottom: Math.round(r.bottom + scrollY), screens: +(y(el) / height).toFixed(2), width: Math.round(r.width), height: Math.round(r.height) };
    };
    const rows = [];
    for (const el of document.querySelectorAll("main h1, main h2, main h3")) if (!el.closest(".northwind, .cf")) rows.push(row(el.tagName.toLowerCase(), el, el.innerText));
    for (const el of document.querySelectorAll("[data-demo]")) rows.push(row("demo", el.querySelector("[role=group]") ?? el, el.dataset.demo));
    for (const el of document.querySelectorAll("[data-copied]")) rows.push(row("copied text", el, el.dataset.copied));
    for (const el of document.querySelectorAll("[data-walk]")) rows.push(row(el.dataset.walk, el, el.innerText || el.dataset.walk));
    for (const el of document.querySelectorAll("video")) if (el.getBoundingClientRect().width > 0) rows.push(row("video", el, el.currentSrc || el.getAttribute("src") || "video"));
    for (const el of document.querySelectorAll('a[href^="/download"], a[href="/privacy"], button[data-copy-link]')) {
      if (el.getBoundingClientRect().width === 0) continue;
      const fixed = (() => { for (let n = el; n && n !== document.body; n = n.parentElement) { const p = getComputedStyle(n).position; if (p === "fixed" || p === "sticky") return true; } return false; })();
      rows.push({ ...row("link", el, `${el.innerText} → ${el.getAttribute("href") ?? "copy link"}`), fixed });
    }
    rows.sort((a, b) => a.y - b.y);
    return { rows, total: document.documentElement.scrollHeight, screens: +(document.documentElement.scrollHeight / height).toFixed(2) };
  }, height);
  report[name] = map;

  // Every screen down, after everything on it has had its turn to play.
  const count = Math.ceil(map.total / height);
  const files = [];
  for (let i = 0; i < count; i++) {
    await page.evaluate((to) => scrollTo(0, to), i * height);
    await page.waitForTimeout(i === 0 ? 200 : 1100);
    const file = `${name}-screen-${String(i + 1).padStart(2, "0")}.jpg`;
    await page.screenshot({ path: `${out}/${file}`, type: "jpeg", quality: 68 });
    files.push(file);
  }
  await page.evaluate(() => scrollTo(0, 0));
  await page.waitForTimeout(500);
  await page.screenshot({ path: `${out}/${name}-full.jpg`, type: "jpeg", quality: 60, fullPage: true });

  // The screens side by side, numbered, as one picture.
  const sheet = await context.newPage();
  const cell = name === "1440" ? 360 : 195;
  await sheet.setViewportSize({ width: 1500, height: 800 });
  await sheet.setContent(`<body style="margin:0;padding:16px;background:#222;font:12px system-ui;color:#fff;display:grid;grid-template-columns:repeat(${name === "1440" ? 4 : 7},${cell}px);gap:12px">${files.map((f, i) => `<figure style="margin:0"><img src="data:image/jpeg;base64,${readFileSync(`${out}/${f}`).toString("base64")}" style="width:${cell}px;display:block"><figcaption>screen ${i + 1} · ${i * height} px</figcaption></figure>`).join("")}</body>`);
  await sheet.waitForTimeout(300);
  await sheet.screenshot({ path: `${out}/${name}-sheet.jpg`, type: "jpeg", quality: 70, fullPage: true });
  await context.close();
}

await browser.close();
writeFileSync(`${out}/map.json`, JSON.stringify(report, null, 1));
for (const [name, map] of Object.entries(report)) {
  console.log(`\n${name}: ${map.total} px, ${map.screens} screens`);
  for (const r of map.rows) console.log(`  ${String(r.y).padStart(6)} px  ${r.screens.toFixed(2).padStart(5)}  ${r.kind.padEnd(12)} ${r.text}${r.fixed ? "  (stays on screen)" : ""}`);
}
