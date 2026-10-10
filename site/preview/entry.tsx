// The home page drawn in the browser with no server behind it, for a preview that is one
// folder of files (scripts/build-preview.mjs). The same components and styles as the site.
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import "@desktop/ui/style.css";
import "../app/globals.css";
import { HomePage } from "../app/HomePage";

// There are no other pages in the folder: links into the site stay where they are.
document.addEventListener("click", (event) => {
  const link = event.target instanceof Element ? event.target.closest("a") : null;
  if (link?.getAttribute("href")?.startsWith("/")) event.preventDefault();
}, true);

/// The hero's three layouts are chosen with ?hero= on the site. A preview has no address to put
/// that in, so it has a small control for it.
function Variants() {
  const [hero, setHero] = useState("a");
  useEffect(() => {
    document.documentElement.dataset.hero = hero;
    // The demos lay themselves out again for the stage's new size.
    window.dispatchEvent(new Event("resize"));
    window.dispatchEvent(new Event("scroll"));
  }, [hero]);
  const NAMES: Record<string, string> = { a: "A: headline, one line and one button above a wide window (the one the site is built with)", b: "B: headline beside a smaller window", c: "C: headline on one line, and the window" };
  return (
    <div style={{ position: "fixed", right: 12, bottom: 12, zIndex: 50, display: "flex", alignItems: "center", gap: 4, padding: "6px 8px 6px 12px", borderRadius: 999, background: "#121213", color: "#fff", font: "500 12.5px system-ui, sans-serif", boxShadow: "0 4px 16px rgba(0,0,0,0.25)" }}>
      <span style={{ marginRight: 4, opacity: 0.7 }}>Hero layout</span>
      {Object.keys(NAMES).map((name) => (
        <button key={name} title={NAMES[name]} aria-pressed={hero === name} data-hero-choice={name} onClick={() => setHero(name)}
          style={{ font: "inherit", fontWeight: 700, width: 28, height: 24, border: 0, borderRadius: 12, cursor: "pointer", background: hero === name ? "#f76808" : "transparent", color: "#fff" }}>
          {name.toUpperCase()}
        </button>
      ))}
    </div>
  );
}

createRoot(document.getElementById("preview") as HTMLElement).render(<><HomePage stars={null} preview /><Variants /></>);
