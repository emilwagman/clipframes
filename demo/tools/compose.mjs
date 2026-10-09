// Composes the site's figures from real renders: the Northwind page (headless Chrome) under the
// app's own views (drawn by Clipframes --render-shots). Nothing is redrawn.
import { chromium } from "playwright-core";
import { writeFileSync } from "node:fs";
const [demo, app, out] = process.argv.slice(2);
const f = (p) => "file://" + p;
const scenes = {
  // Element: the real overlay over the page, at the size of the screen it was drawn for.
  "element": `<img src="${f(demo + "/page.png")}" class="full"><img src="${f(app + "/overlay-element.png")}" class="full">`,
  "screenshot": `<img src="${f(demo + "/page.png")}" class="full"><img src="${f(app + "/overlay-screenshot.png")}" class="full">`,
  "clip": `<img src="${f(demo + "/frame-4.png")}" class="full"><img src="${f(app + "/overlay-clip.png")}" class="full">`,
  // The shortcut opens the bar over whatever you're in.
  "bar": `<img src="${f(demo + "/page.png")}" class="full"><img src="${f(app + "/bar-keys.png")}" style="position:absolute;left:50%;bottom:26px;transform:translateX(-50%);width:443px">`,
  // After a capture: the toast.
  "copied": `<img src="${f(demo + "/page.png")}" class="full"><img src="${f(app + "/toast-copied.png")}" style="position:absolute;left:50%;bottom:60px;transform:translateX(-50%);width:318px">`,
  // The library window on a quiet desktop.
  "library": `<div style="position:absolute;inset:0;background:#e9edf2"></div><img src="${f(app + "/library.png")}" style="position:absolute;left:160px;top:90px;width:1120px;border-radius:12px;box-shadow:0 0 0 1px rgba(0,0,0,.25),0 30px 70px -20px rgba(0,0,0,.45)">`,
  "onboarding": `<div style="position:absolute;inset:0;background:#e9edf2"></div><img src="${f(app + "/onboarding-welcome.png")}" style="position:absolute;left:340px;top:160px;width:760px;border-radius:12px;box-shadow:0 0 0 1px rgba(0,0,0,.25),0 30px 70px -20px rgba(0,0,0,.45)">`,
};
const b = await chromium.launch({ executablePath: "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" });
const p = await b.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 });
for (const [name, html] of Object.entries(scenes)) {
  const file = `${out}/${name}.html`;
  writeFileSync(file, `<!doctype html><style>body{margin:0}.stage{position:relative;width:1440px;height:900px;overflow:hidden}.full{position:absolute;inset:0;width:1440px;height:900px}</style><div class="stage">${html}</div>`);
  await p.goto("file://" + file, { waitUntil: "load" });
  const broken = await p.evaluate(() => [...document.images].filter((i) => !i.naturalWidth).map((i) => i.src));
  if (broken.length) console.log(name, "missing", broken);
  await p.screenshot({ path: `${out}/${name}.jpg`, type: "jpeg", quality: 86 });
}
await b.close();
console.log("composed", Object.keys(scenes).join(", "));
