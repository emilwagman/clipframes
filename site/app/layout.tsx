import type { Metadata, Viewport } from "next";
import { Caveat } from "next/font/google";
import "./globals.css";
import { SITE_NAME, SITE_URL } from "@/lib/site";

const description = "Point at the thing you want changed, and Clipframes tells your agent exactly what it is. A Mac app for building with Claude Code and Codex.";

// The handwriting for notes and marks, served from this site.
const hand = Caveat({ subsets: ["latin"], weight: ["700"], variable: "--hand", display: "swap" });

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
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#faf8f5" },
    { media: "(prefers-color-scheme: dark)", color: "#171513" },
  ],
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={hand.variable}>
      <body>{children}</body>
    </html>
  );
}
