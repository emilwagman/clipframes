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
  return <div key={path}>{PAGES[path]()}</div>;
}

createRoot(document.getElementById("preview") as HTMLElement).render(<Preview />);
