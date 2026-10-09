"use client";

import { usePathname } from "next/navigation";
import { useEffect } from "react";
import { track } from "@/lib/analytics";

/// Counts a view of each page. Does nothing when the site was built without a key (lib/analytics.ts).
export default function Analytics() {
  const path = usePathname();
  useEffect(() => track("$pageview"), [path]);
  return null;
}
