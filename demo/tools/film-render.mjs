// Renders the launch film (desktop/film.html) frame by frame: steps the page's own clock one
// frame at a time and pipes a picture of each to ffmpeg. Needs the dev server:
//   cd desktop && pnpm vite --port 1437 --strictPort
//   node film-render.mjs --out film.mp4 [--url http://localhost:1437/film.html] [--scale 2]
//                        [--to 5.6] [--end 16] [--window browser] [--stills 3.3,5.0 --dir out/]
import { spawn } from "node:child_process";
import { once } from "node:events";
import { chromium } from "playwright-core";

const args = Object.fromEntries(process.argv.slice(2).join(" ").split("--").filter(Boolean).map((a) => a.trim().split(/\s+/)).map(([k, v]) => [k, v ?? "1"]));
const scale = Number(args.scale ?? 1);
const query = new URLSearchParams({ render: "1" });
if (args.window) query.set("window", args.window);
if (args.look) query.set("look", args.look);
if (args.end) query.set("end", args.end);
const url = `${args.url ?? "http://localhost:1437/film.html"}?${query}`;

const browser = await chromium.launch({ channel: "chrome", headless: true });
const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: scale });
page.on("pageerror", (error) => console.error("page error:", error.message));
// Vite answers 504 while it is still finding a new page's dependencies; the next load works.
for (let tries = 0; ; tries++) {
  await page.goto(url, { waitUntil: "networkidle" });
  if (await page.waitForFunction(() => document.body.dataset.ready === "1", null, { timeout: 4000 }).catch(() => null)) break;
  if (tries === 5) throw new Error("the film page did not start");
}
const { fps, frames } = await page.evaluate(() => ({ fps: window.film.fps, frames: window.film.frames }));
const last = Math.min(frames, args.to ? Math.round(Number(args.to) * fps) : frames);
const stills = args.stills ? args.stills.split(",").map((t) => Math.round(Number(t) * fps)) : null;

let ffmpeg = null;
if (!stills) {
  ffmpeg = spawn("ffmpeg", ["-y", "-loglevel", "error", "-f", "image2pipe", "-framerate", String(fps), "-c:v", "png", "-i", "-", "-c:v", "libx264", "-preset", "slow", "-crf", "14", "-threads", "4", "-pix_fmt", "yuv420p", "-vf", "scale=out_color_matrix=bt709:out_range=tv", "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-movflags", "+faststart", args.out], { stdio: ["pipe", "inherit", "inherit"] });
}
const started = Date.now();
for (let n = 0; n < last; n++) {
  await page.evaluate((n) => window.film.step(n), n);
  if (stills) {
    if (stills.includes(n)) await page.screenshot({ path: `${args.dir}/t${(n / fps).toFixed(2)}.png` });
    if (n >= Math.max(...stills)) break;
    continue;
  }
  const picture = await page.screenshot({ type: "png" });
  if (!ffmpeg.stdin.write(picture)) await once(ffmpeg.stdin, "drain");
  if (n % 120 === 0) console.error(`frame ${n}/${last}, ${((Date.now() - started) / 1000).toFixed(0)} s`);
}
console.log(JSON.stringify(await page.evaluate(() => window.film.result()), null, 2));
await browser.close();
if (ffmpeg) {
  ffmpeg.stdin.end();
  await once(ffmpeg, "close");
}
