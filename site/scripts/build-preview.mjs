// Builds the home page as a folder that works from any address with no server behind it:
// index.html (the styles inline, the markup the page is drawn into) and preview.js beside it.
// It is for looking at the page from a link, e.g. as a private Claude artifact; nothing here
// deploys anything.
//
//   node scripts/build-preview.mjs <folder to write>
//
// index.html has no <html>, <head> or <body> of its own: the host that shows it wraps it in those.
import { mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const site = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const out = path.resolve(process.argv[2] ?? path.join(site, "preview/dist"));
const { transform } = createRequire(import.meta.url)("../lib/app-style.cjs");
const at = (file) => path.join(site, file);

const result = await build({
  entryPoints: [at("preview/entry.tsx")],
  bundle: true,
  minify: true,
  write: false,
  outdir: out,
  entryNames: "preview",
  format: "iife",
  target: "es2022",
  jsx: "automatic",
  // The site's tsconfig maps packages to their types, which is not what a bundle wants.
  tsconfigRaw: { compilerOptions: { jsx: "react-jsx" } },
  define: { "process.env.NODE_ENV": '"production"', "process.env.NEXT_PUBLIC_POSTHOG_KEY": '""' },
  alias: {
    "@/lib/analytics": at("preview/no-analytics.ts"),
    "@": site,
    "@desktop": path.join(site, "../desktop"),
    "@tauri-apps/api/core": at("lib/no-core.ts"),
    // One copy of each, the site's, also for the files that live in ../desktop.
    react: at("node_modules/react"),
    "react-dom": at("node_modules/react-dom"),
    zustand: at("node_modules/zustand"),
  },
  plugins: [{
    name: "as-the-site-does",
    setup(b) {
      // The app's stylesheet, kept to the app's interface, as next.config.ts does it.
      b.onLoad({ filter: /desktop[\\/]ui[\\/]style\.css$/ }, (args) => ({ contents: transform(readFileSync(args.path, "utf8")), loader: "css" }));
      // The recordings are inside their addresses too: the same list as lib/media.ts, with each file in place of its path.
      b.onLoad({ filter: /lib[\\/]media\.ts$/ }, (args) => {
        const source = readFileSync(args.path, "utf8").replace(/"(\/recordings\/[^"]+)"/g, (_, file) => {
          const type = file.endsWith(".mp4") ? "video/mp4" : "image/jpeg";
          return JSON.stringify(`data:${type};base64,${readFileSync(at(`public${file}`)).toString("base64")}`);
        });
        return { contents: source, loader: "ts" };
      });
      // A picture is imported as { src }, as Next does it; here the picture is inside the address.
      b.onLoad({ filter: /\.png$/ }, (args) => ({ contents: `export default { src: "data:image/png;base64,${readFileSync(args.path).toString("base64")}" };`, loader: "js" }));
    },
  }],
});

const file = (ending) => result.outputFiles.find((f) => f.path.endsWith(ending)).text;
const css = file(".css");
if (css.includes("</style")) throw new Error("The styles would end their own <style> element.");

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
writeFileSync(path.join(out, "preview.js"), file(".js"));
writeFileSync(path.join(out, "index.html"), `<title>Clipframes Site Preview</title>
<style>
html, body { background: #fafaf9; }
${css}</style>
<div id="preview"></div>
<script src="preview.js"></script>
`);
for (const name of ["index.html", "preview.js"]) console.log(`${name}  ${(statSync(path.join(out, name)).size / 1024).toFixed(0)} kB`);
