// Packs three of the lab's pages into one self-contained HTML file each, to try from a link
// with no dev server: script, styles, pictures and the demo page are all inside the file.
//
//   cd desktop && node tools/build-try.mjs [out-folder]
//
// The files have no <html>, <head> or <body> of their own: the host that shows them adds those.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "vite";

const desktop = join(dirname(fileURLToPath(import.meta.url)), "..");
const out = resolve(process.argv[2] ?? join(desktop, "dist-try"));
const pages = [
  { file: "today.html", entry: "today.tsx", title: "Clipframes Today", name: "Clipframes today", button: "Open Clipframes", line: "Click Open, click anything on the page, type what should change and press Enter. Esc closes." },
  { file: "sentence.html", entry: "sentence.tsx", title: "Clipframes Write and Point", name: "Write and point", button: "Open", line: "Click Open and start writing. Click things on the page as you write. Enter copies." },
  { file: "tray.html", entry: "tray.tsx", title: "Clipframes Tray", name: "The tray", button: "Start pointing", line: "Hold Option (Alt on Windows) and click things, or use the button in place of the key. Then write in the tray." },
];
const attr = (text) => text.replace(/&/g, "&amp;").replace(/"/g, "&quot;");
const northwind = readFileSync(join(desktop, "lab/northwind.html"), "utf8");
// What the one-file pages add around the stage: a strip that says what to do, with a button
// in place of the global shortcut, and a note for screens too narrow to point on.
const extra = `
html, body { height: 100%; background: #fff; }
:root { color-scheme: dark; }
#strip { position: absolute; left: 0; right: 0; top: 0; height: 48px; z-index: 10; display: flex; align-items: center; gap: 14px; padding: 0 10px 0 16px; background: #121213; color: #fff; font: 13px/1.3 var(--sans); }
#strip b { flex: none; font-weight: 600; }
#strip span { flex: 1; min-width: 0; color: #9c9da5; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
#strip button { flex: none; height: 32px; padding: 0 16px; border-radius: 16px; background: #f76808; color: #fff; font-weight: 600; cursor: pointer; }
#strip button.on { background: #fff; color: #121213; }
#stage { position: absolute; inset: 48px 0 0 0; overflow: hidden; background: #fff; }
#stage #open { display: none; }
#small { display: none; }
@media (max-width: 700px) {
  #strip span { display: none; }
  #small { display: block; position: absolute; left: 12px; right: 12px; top: 60px; z-index: 10; padding: 10px 14px; border-radius: 12px; background: #121213; color: #fff; font: 13px/1.4 var(--sans); }
  #clip { width: auto; right: 16px; }
}`;

mkdirSync(out, { recursive: true });
for (const page of pages) {
  const result = await build({
    root: desktop,
    configFile: false,
    logLevel: "warn",
    define: { "process.env.NODE_ENV": '"production"' },
    build: { write: false, target: "es2022", minify: true, cssCodeSplit: false, assetsInlineLimit: 100_000_000, lib: { entry: join(desktop, "lab/proto", page.entry), formats: ["es"], fileName: "page" }, rollupOptions: { output: { codeSplitting: false } } },
  });
  const files = (Array.isArray(result) ? result : [result]).flatMap((r) => r.output);
  const script = files.filter((f) => f.type === "chunk").map((f) => f.code).join("\n").replace(/<\/script/g, "<\\/script");
  const styles = files.filter((f) => f.type === "asset" && f.fileName.endsWith(".css")).map((f) => String(f.source)).join("\n");
  const html = `<title>${page.title}</title>
<style>${styles}${extra}</style>
<div id="strip"><b>${page.name}</b><span>${page.line}</span><button id="start" data-shortcut>${page.button}</button></div>
<div id="small">This is meant for a desktop screen with a mouse. On a screen this narrow it can be looked at but not really used.</div>
<div id="stage"><iframe class="page" title="Demo page" srcdoc="${attr(northwind)}"></iframe><div id="glass"></div><button id="open" hidden></button><div id="clip" hidden><span>On the clipboard</span><pre></pre></div></div>
<script type="module">${script}</script>
`;
  if (/\b(src|href)="(https?:|\/)/.test(html.replace(/srcdoc="[^"]*"/, ""))) throw new Error(`${page.file} refers to something outside itself`);
  writeFileSync(join(out, page.file), html);
  console.log(`${page.file}  ${(html.length / 1024).toFixed(0)} KB`);
}
