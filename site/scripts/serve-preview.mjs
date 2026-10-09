// Serves a built preview (scripts/build-preview.mjs) the way an artifact host does: index.html
// wrapped in the host's page skeleton, from a nested address, so a path that only works from
// the root of a site would break here too.
//
//   node scripts/serve-preview.mjs <folder> [port]     then open http://localhost:5232/some/deep/path/
import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import path from "node:path";

const [folder, port = "5232"] = process.argv.slice(2);
const BASE = "/some/deep/path/";
const TYPES = { ".js": "text/javascript", ".css": "text/css", ".png": "image/png", ".svg": "image/svg+xml" };

createServer((request, response) => {
  const { pathname } = new URL(request.url, "http://localhost");
  if (!pathname.startsWith(BASE) || pathname.includes("..")) return void response.writeHead(404).end("Not in the preview.");
  const name = pathname.slice(BASE.length) || "index.html";
  try {
    const body = readFileSync(path.join(folder, name));
    if (name !== "index.html") return void response.writeHead(200, { "content-type": TYPES[path.extname(name)] ?? "application/octet-stream" }).end(body);
    response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    response.end(`<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"></head><body>${body}</body></html>`);
  } catch {
    response.writeHead(404).end("Not in the preview.");
  }
}).listen(Number(port), () => console.log(`http://localhost:${port}${BASE}`));
