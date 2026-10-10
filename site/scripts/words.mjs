// Counts the words a visitor is given to read on a page, screen by screen: every visible word
// outside the demos' own made-up apps and the app's interface. Headless.
//
//   node scripts/words.mjs <url> [width] [height]
import { chromium } from "playwright-core";

const [url, width = "1440", height = "900"] = process.argv.slice(2);
const browser = await chromium.launch({ channel: "chrome", headless: true });
const page = await browser.newPage({ viewport: { width: +width, height: +height } });
await page.goto(url, { waitUntil: "networkidle" });
await page.waitForTimeout(800);
const counts = await page.evaluate((height) => {
  const screens = [];
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  for (let node; (node = walker.nextNode()); ) {
    const el = node.parentElement;
    if (!el || el.closest(".northwind, .focusapp, .cf, video, script, style, [data-agent], [data-hero-choice]")) continue;
    const box = el.getBoundingClientRect();
    if (box.width === 0 || getComputedStyle(el).visibility === "hidden") continue;
    // The top bar stays on screen; it is counted once, in the first screen.
    const fixed = el.closest("header") !== null;
    const screen = fixed ? 0 : Math.floor((box.top + scrollY) / height);
    screens[screen] = (screens[screen] ?? 0) + node.textContent.split(/\s+/).filter(Boolean).length;
  }
  return Array.from(screens, (n) => n ?? 0);
}, +height);
await browser.close();
console.log(`${counts.reduce((a, b) => a + b, 0)} words; by screen: ${counts.join(", ")}`);
