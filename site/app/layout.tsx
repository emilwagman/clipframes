import type { Metadata, Viewport } from "next";
import "./globals.css";
import { SITE_NAME, SITE_URL } from "@/lib/site";

const description = "Point at the thing you want changed, and Clipframes tells your agent exactly what it is. A Mac app for building with Claude Code and Codex.";

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: SITE_NAME,
  description,
  applicationName: SITE_NAME,
  referrer: "no-referrer",
  icons: { icon: "/icon.svg" },
  openGraph: { title: SITE_NAME, description, url: SITE_URL, siteName: SITE_NAME, type: "website" },
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#faf8f5" },
    { media: "(prefers-color-scheme: dark)", color: "#171513" },
  ],
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
