// Clipframes running for real over the demo page, in a browser: desktop/demo.html. The picker
// is ui/web.ts; everything drawn is the app's own components.
import { createRoot } from "react-dom/client";
import "../ui/style.css";
import { connect } from "../ui/store";
import { OnPage } from "../ui/OnPage";
import { webPlatform } from "../ui/web";

const query = new URLSearchParams(location.search);
if (query.get("look")) document.documentElement.dataset.look = query.get("look") as string;

const frame = document.getElementById("page") as HTMLIFrameElement;
const glass = document.getElementById("glass") as HTMLElement;
const open = document.getElementById("open") as HTMLButtonElement;
const copied = document.getElementById("copied") as HTMLElement;

frame.addEventListener("load", () => {
  const page = frame.contentDocument as Document;
  const platform = webPlatform({
    glass,
    elementAt: (x, y) => page.elementFromPoint(x, y),
    rectOf: (element) => {
      const r = element.getBoundingClientRect();
      return { x: r.x, y: r.y, width: r.width, height: r.height };
    },
    place: "Google Chrome · localhost:3000",
    where: 'Google Chrome "Invoices"',
  });
  connect(platform);
  createRoot(document.createElement("div")).render(<OnPage platform={platform} glass={glass} />);
  platform.onClose((text) => {
    open.hidden = false;
    copied.hidden = text === "";
    copied.textContent = text;
  });
  const start = () => {
    open.hidden = true;
    copied.hidden = true;
    platform.open();
  };
  open.addEventListener("click", start);
  if (!query.has("closed")) start();
  document.body.dataset.ready = "1";
});
