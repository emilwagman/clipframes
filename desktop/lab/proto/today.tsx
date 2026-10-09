// Today's Clipframes, unchanged (ui/web.ts, ui/OnPage.tsx), on the prototypes' stage: the
// version the alternatives are compared against.
import { createRoot } from "react-dom/client";
import { connect } from "../../ui/store";
import { OnPage } from "../../ui/OnPage";
import { webPlatform } from "../../ui/web";
import { lab, SHORTCUT } from "./stage";

const stage = await lab();
const platform = webPlatform({ glass: stage.glass, elementAt: stage.elementAt, rectOf: stage.rectOf, place: "Google Chrome · localhost:3000", where: 'Google Chrome "Invoices"' }, { shortcut: SHORTCUT });
connect(platform);
createRoot(document.createElement("div")).render(<OnPage platform={platform} glass={stage.glass} />);

let picking = false;
platform.onRound((round) => {
  picking = round.picking;
  // The app rewrites the clipboard after every pick (app.rs, publish).
  if (round.reference) stage.clipboard.set(round.reference);
  stage.hint(round.picking ? "" : `${SHORTCUT} opens Clipframes`);
});
stage.onShortcut(() => (picking ? void platform.done() : platform.open()));
stage.hint(`${SHORTCUT} opens Clipframes`);
stage.ready();
