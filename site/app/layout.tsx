import type { Metadata, Viewport } from "next";
// The app's own stylesheet, kept to the app's interface (lib/app-style.cjs). It comes first:
// the site's colours are named after the app's.
import "@desktop/ui/style.css";
import "./globals.css";
import { SITE_NAME, SITE_URL } from "@/lib/site";

const description = "Point at the thing you want changed, and Clipframes tells your agent exactly what it is. For Mac and Windows, for building with Claude Code and Codex.";

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: SITE_NAME,
  description,
  applicationName: SITE_NAME,
  referrer: "no-referrer",
  icons: { icon: [{ url: "/icon-64.png", sizes: "64x64", type: "image/png" }, { url: "/icon.png", sizes: "512x512", type: "image/png" }], apple: "/apple-icon.png" },
  openGraph: { title: SITE_NAME, description, url: SITE_URL, siteName: SITE_NAME, type: "website" },
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  themeColor: "#fafaf9",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
