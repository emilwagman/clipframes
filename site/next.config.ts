import path from "node:path";
import type { NextConfig } from "next";

// Next runs from this folder.
const here = (file: string) => path.resolve(process.cwd(), file);

// A static page with inline Next.js scripts, and recordings served from this site.
const csp = [
  "default-src 'self'",
  // Next's development server runs the page's modules through eval; the built site does not.
  `script-src 'self' 'unsafe-inline'${process.env.NODE_ENV === "development" ? " 'unsafe-eval'" : ""}`,
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data:",
  "media-src 'self'",
  "font-src 'self'",
  // The site's anonymous counts go to PostHog in the EU, and only when a key is set (lib/analytics.ts).
  `connect-src 'self'${process.env.NEXT_PUBLIC_POSTHOG_KEY ? " https://eu.i.posthog.com" : ""}`,
  "object-src 'none'",
  "base-uri 'none'",
  "form-action 'none'",
  "frame-ancestors 'none'",
].join("; ");

const security = [
  { key: "Referrer-Policy", value: "no-referrer" },
  { key: "X-Frame-Options", value: "DENY" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "Content-Security-Policy", value: csp },
  { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=(), interest-cohort=()" },
];

const config: NextConfig = {
  poweredByHeader: false,
  // The demos mount the app's own interface, which lives beside the site in ../desktop.
  experimental: { externalDir: true },
  outputFileTracingRoot: here(".."),
  webpack(config) {
    config.resolve.alias = {
      ...config.resolve.alias,
      // One copy of the store's library, the site's, whether or not ../desktop has its own installed.
      zustand$: here("node_modules/zustand"),
      // History asks the desktop core for its captures; on the site it is handed them instead.
      "@tauri-apps/api/core$": here("lib/no-core.ts"),
    };
    // The app's stylesheet is kept to the app's interface (lib/app-style.cjs).
    config.module.rules.push({ test: /desktop[\\/]ui[\\/]style\.css$/, enforce: "pre", use: [here("lib/app-style.cjs")] });
    return config;
  },
  async headers() {
    return [{ source: "/:path*", headers: security }];
  },
};
export default config;
