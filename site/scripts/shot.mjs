// One picture of the home page at a size, after a wait, for looking at a single state.
//
//   node scripts/shot.mjs <url> <file.jpg> <width> <height> [phone] [ms to wait] [scroll to px]
import { chromium } from "playwright-core";
const [url, file, width, height, kind = "", wait = "6500", y = "0"] = process.argv.slice(2);
const browser = await chromium.launch({ channel: "chrome", headless: true });
const page = await browser.newPage({ viewport: { width: +width, height: +height }, ...(kind === "phone" ? { hasTouch: true, isMobile: true, deviceScaleFactor: 2 } : {}) });
await page.goto(url, { waitUntil: "networkidle" });
if (+y) await page.evaluate((y) => scrollTo(0, y), +y);
await page.waitForTimeout(+wait);
await page.screenshot({ path: file, type: "jpeg", quality: 75 });
await browser.close();
