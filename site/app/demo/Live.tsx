"use client";

import { useEffect } from "react";
import { OnPage } from "@desktop/ui/OnPage";
import { demo, onLive, useSite } from "./engine";

/// The app's interface, drawn over whichever demo is live. There is one of these on the page.
export default function Live() {
  const id = useSite((site) => site.live);
  // After the interface's own effects, so it is listening when the picker speaks.
  useEffect(() => {
    if (id) onLive(id);
  }, [id]);
  const d = id && demo(id);
  return d ? <OnPage key={id} platform={d.platform} glass={d.glass} /> : null;
}
