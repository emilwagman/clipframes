// The site drawn in the browser with no server behind it, for a preview that is one folder of
// files (scripts/build-preview.mjs). The same components and styles as the site.
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import "@desktop/ui/style.css";
import "../app/globals.css";
import { HomePage } from "../app/HomePage";
import { ComparePage } from "../app/compare/ComparePage";

/// The pages the preview holds. A link to one of them shows it; a link to any other page of the
/// site stays where it is, since that page is not in the folder.
const PAGES: Record<string, () => React.ReactNode> = {
  "/": () => <HomePage stars={null} preview />,
  "/compare": () => <ComparePage stars={null} preview />,
};

/// The scene's three arrangements are chosen with ?scene= on the site. A preview has no address
/// to put that in, so it has a small control for it.
function Layouts() {
  const [layout, setLayout] = useState("a");
  useEffect(() => {
    document.documentElement.dataset.sceneLayout = layout;
    // The app's interface lays itself out again for the scene's new shape.
    window.dispatchEvent(new Event("resize"));
  }, [layout]);
  const NAMES: Record<string, string> = { a: "A: the agent in a narrow column beside the thing (the one the site is built with)", b: "B: the agent as a strip under the thing", c: "C: two overlapping windows" };
  return (
    <div style={{ position: "fixed", right: 12, bottom: 12, zIndex: 50, display: "flex", alignItems: "center", gap: 4, padding: "6px 8px 6px 12px", borderRadius: 999, background: "#121213", color: "#fff", font: "500 12.5px system-ui, sans-serif", boxShadow: "0 4px 16px rgba(0,0,0,0.25)" }}>
      <span style={{ marginRight: 4, opacity: 0.7 }}>Scene</span>
      {Object.keys(NAMES).map((name) => (
        <button key={name} title={NAMES[name]} aria-pressed={layout === name} data-layout-choice={name} onClick={() => setLayout(name)}
          style={{ font: "inherit", fontWeight: 700, width: 28, height: 24, border: 0, borderRadius: 12, cursor: "pointer", background: layout === name ? "#f76808" : "transparent", color: "#fff" }}>
          {name.toUpperCase()}
        </button>
      ))}
    </div>
  );
}

function Preview() {
  const [path, setPath] = useState("/");
  useEffect(() => {
    const onClick = (event: MouseEvent) => {
      const href = event.target instanceof Element ? event.target.closest("a")?.getAttribute("href") : null;
      if (!href?.startsWith("/")) return;
      event.preventDefault();
      if (!(href in PAGES)) return;
      setPath(href);
      scrollTo(0, 0);
    };
    document.addEventListener("click", onClick, true);
    return () => document.removeEventListener("click", onClick, true);
  }, []);
  // A page is drawn from nothing each time it is shown, as it is on the site.
  return <><div key={path}>{PAGES[path]()}</div>{path === "/" && <Layouts />}</>;
}

createRoot(document.getElementById("preview") as HTMLElement).render(<Preview />);
