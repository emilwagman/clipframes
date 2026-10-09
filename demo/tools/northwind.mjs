// Renders the Northwind demo page at 1440 × 900 @2x: the full page, crops for the demo library,
// six "frames" of a short session, and the measured frames of the elements the overlay points at.
import { chromium } from "playwright-core";
const [file, out] = process.argv.slice(2);
const b = await chromium.launch({ executablePath: "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" });
const p = await b.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 });
await p.goto("file://" + file);
const box = async (sel) => p.evaluate((s) => { const r = document.querySelector(s).getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height }; }, sel);
const btn = await box("#new-invoice");
const stats = await box(".stats");
console.log(JSON.stringify({ button: btn, stats }));
await p.screenshot({ path: `${out}/page.png` });
await p.screenshot({ path: `${out}/element.png`, clip: { x: btn.x - 40, y: btn.y - 24, width: btn.w + 80, height: btn.h + 48 } });
await p.screenshot({ path: `${out}/area.png`, clip: { x: stats.x - 10, y: stats.y - 10, width: stats.w + 20, height: stats.h + 20 } });
// A short session: hover, press, type a search, hover a row.
const steps = [
  async () => {},
  async () => { await p.hover("#new-invoice"); },
  async () => { await p.click("#invoice-search"); },
  async () => { await p.keyboard.type("Maer"); },
  async () => { await p.keyboard.type("sk"); },
  async () => { await p.hover("#invoice-table tbody tr"); },
];
for (let i = 0; i < steps.length; i++) { await steps[i](); await p.screenshot({ path: `${out}/frame-${i + 1}.png` }); }
await b.close();
