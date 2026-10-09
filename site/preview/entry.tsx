// The home page drawn in the browser with no server behind it, for a preview that is one
// folder of files (scripts/build-preview.mjs). The same components and styles as the site.
import { createRoot } from "react-dom/client";
import "@desktop/ui/style.css";
import "../app/globals.css";
import { HomePage } from "../app/HomePage";

// There are no other pages in the folder: links into the site stay where they are.
document.addEventListener("click", (event) => {
  const link = event.target instanceof Element ? event.target.closest("a") : null;
  if (link?.getAttribute("href")?.startsWith("/")) event.preventDefault();
}, true);

createRoot(document.getElementById("preview") as HTMLElement).render(<HomePage stars={null} preview />);
