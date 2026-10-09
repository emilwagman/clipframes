import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { describe, expect, it } from "vitest";

const { transform } = createRequire(import.meta.url)("./app-style.cjs") as { transform(css: string): string };

describe("the app's stylesheet on the site", () => {
  it("moves every rule under the scope", () => {
    const out = transform("button.pill { height: 32px; }\n.bar, .note { position: absolute; }");
    expect(out).toContain(".cf button.pill { height: 32px; }");
    expect(out).toContain(".cf .bar, .cf .note { position: absolute; }");
  });

  it("publishes the app's tokens to the site and keeps them in the scope", () => {
    const out = transform(":root { color-scheme: dark; --accent: #f76808; --ring: 0 0 0 1px var(--accent); }");
    expect(out).toContain(":root {\n  --app-accent: #f76808;\n  --app-ring: 0 0 0 1px var(--app-accent);\n}");
    expect(out).toContain(".cf { color-scheme: dark; --accent: #f76808;");
  });

  it("leaves the page alone", () => {
    const out = transform("html, body { overflow: hidden; }\nhtml.window, html.window body { background: black; }\n:root[data-look=\"blue\"] { --accent: blue; }\nbody { font: 13px sans-serif; }\n* { box-sizing: border-box; }");
    expect(out).not.toContain("overflow: hidden");
    expect(out).not.toContain("black");
    expect(out).not.toContain("blue");
    expect(out).toContain(".cf { font: 13px sans-serif; }");
    expect(out).toContain(".cf, .cf * { box-sizing: border-box; }");
  });

  it("does not touch the steps of an animation, and reaches into media rules", () => {
    const out = transform("@keyframes pulse { 50% { opacity: 0.35; } }\n@media (prefers-reduced-motion: reduce) { .dot { animation: none; } }");
    expect(out).toContain("@keyframes pulse { 50% { opacity: 0.35; } }");
    expect(out).toContain(".cf .dot { animation: none; }");
  });

  it("handles the real file: no rule escapes the scope", () => {
    const out = transform(readFileSync(new URL("../../desktop/ui/style.css", import.meta.url), "utf8"));
    expect(out).toContain("--app-accent: #f76808;");
    const escaped = out
      .replace(/\/\*[\s\S]*?\*\//g, "")
      .replace(/@keyframes[^{]+\{(?:[^{}]*\{[^{}]*\})*[^{}]*\}/g, "")
      .split("}")
      .map((chunk) => chunk.split("{")[0].replace(/@media[^{]*$/, "").trim())
      .filter((selector) => selector && !selector.startsWith("@") && selector !== ":root")
      .filter((selector) => !selector.split(",").every((one) => one.trim().startsWith(".cf")));
    expect(escaped).toEqual([]);
  });
});
