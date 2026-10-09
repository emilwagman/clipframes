// Takes a full-page picture of the site at a desktop and a phone width, for looking it over.
//   node site-shot.mjs http://localhost:5330 /tmp/out
import { chromium } from "playwright-core";
const [url, out] = process.argv.slice(2);
const browser = await chromium.launch({ channel: "chrome", headless: true });
for (const [name, width] of [["desktop", 1280], ["phone", 400]]) {
  const page = await browser.newPage({ viewport: { width, height: 900 } });
  await page.goto(url, { waitUntil: "networkidle" });
  // Scroll through once so lazy pictures load.
  await page.evaluate(async () => { for (let y = 0; y < document.body.scrollHeight; y += 600) { scrollTo(0, y); await new Promise((r) => setTimeout(r, 80)); } scrollTo(0, 0); });
  await page.waitForTimeout(400);
  console.log(name, "sideways scroll:", await page.evaluate(() => document.documentElement.scrollWidth > innerWidth));
  await page.screenshot({ path: `${out}/site-${name}.png`, fullPage: true });
  await page.close();
}
await browser.close();
