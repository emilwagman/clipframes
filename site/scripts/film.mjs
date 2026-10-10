// Films the first screen's scene: a short video of one example playing, and stills at chosen
// moments. Headless; nothing appears on screen. The video is made of pictures taken as fast as
// the browser gives them (about ten a second), each held for as long as it was on screen.
//
//   node scripts/film.mjs <url> <folder> <name> <width> <height> [phone] [case to choose] [seconds] [stills at ms, comma separated]
//
// Needs Google Chrome, and ffmpeg for the video.
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { chromium } from "playwright-core";

const [url, folder, name, width, height, kind = "", pick = "", seconds = "14", stills = ""] = process.argv.slice(2);
const out = path.resolve(folder);
const raw = path.join(out, `raw-${name}`);
mkdirSync(raw, { recursive: true });
const browser = await chromium.launch({ channel: "chrome", headless: true });
const page = await browser.newPage({ viewport: { width: +width, height: +height }, ...(kind === "phone" ? { hasTouch: true, isMobile: true, deviceScaleFactor: 2 } : {}) });
await page.goto(url, { waitUntil: "networkidle" });
if (pick) await page.locator(`[data-case="${pick}"]`).click();
const start = Date.now();
const frames = [];
while (Date.now() - start < +seconds * 1000) {
  const at = Date.now() - start;
  const file = path.join(raw, `${String(frames.length).padStart(4, "0")}.jpg`);
  await page.screenshot({ path: file, type: "jpeg", quality: 80 });
  frames.push({ at, file });
}
await browser.close();
for (const want of stills.split(",").filter(Boolean).map(Number)) {
  const nearest = frames.reduce((best, frame) => (Math.abs(frame.at - want) < Math.abs(best.at - want) ? frame : best));
  copyFileSync(nearest.file, path.join(out, `${name}-${String(want).padStart(5, "0")}.jpg`));
}
const list = frames.map((frame, i) => `file '${frame.file}'\nduration ${(((frames[i + 1]?.at ?? frame.at + 100) - frame.at) / 1000).toFixed(3)}`).join("\n");
writeFileSync(path.join(raw, "list.txt"), `${list}\nfile '${frames.at(-1).file}'\n`);
execFileSync("ffmpeg", ["-v", "error", "-y", "-f", "concat", "-safe", "0", "-i", path.join(raw, "list.txt"), "-vf", "fps=20,scale=trunc(iw/2)*2:trunc(ih/2)*2", "-an", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "24", "-movflags", "+faststart", path.join(out, `${name}.mp4`)]);
rmSync(raw, { recursive: true, force: true });
console.log(`${path.join(out, `${name}.mp4`)}  ${frames.length} pictures in ${seconds} s`);
