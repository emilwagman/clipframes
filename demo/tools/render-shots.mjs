// Renders the pictures the website shows, from the app's real interface: the lab page
// (desktop/lab.html) mounts the same React components over the Northwind demo page.
//
//   cd demo/tools && npm install && node render-shots.mjs
//
// Needs Google Chrome. Runs headless; nothing appears on screen.
import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const here = dirname(fileURLToPath(import.meta.url));
const desktop = join(here, "../../desktop");
const out = join(here, "../../site/public/shots");
const port = 5321;
// What the site calls each picture, and the moment of the app it shows.
const shots = { element: "empty", pick: "picked", screenshot: "area", clip: "recording", history: "history" };

mkdirSync(out, { recursive: true });
const vite = spawn("npx", ["vite", "--port", String(port), "--strictPort"], { cwd: desktop, stdio: "ignore" });
try {
  await new Promise((r) => setTimeout(r, 3000));
  const browser = await chromium.launch({ channel: "chrome", headless: true });
  const page = await browser.newPage({ viewport: { width: 1120, height: 700 }, deviceScaleFactor: 2 });
  for (const [name, state] of Object.entries(shots)) {
    await page.goto(`http://localhost:${port}/lab.html?state=${state}`);
    await page.waitForSelector("body[data-ready]");
    await page.waitForTimeout(300);
    await page.screenshot({ path: join(out, `${name}.jpg`), type: "jpeg", quality: 88 });
    console.log(`${name}.jpg`);
  }
  await browser.close();
} finally {
  vite.kill();
}
