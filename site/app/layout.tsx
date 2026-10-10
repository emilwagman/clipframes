import type { Metadata, Viewport } from "next";
// The app's own stylesheet, kept to the app's interface (lib/app-style.cjs). It comes first:
// the site's colours are named after the app's.
import "@desktop/ui/style.css";
import "./globals.css";
import Analytics from "./Analytics";
import { SITE_NAME, SITE_URL } from "@/lib/site";

const description = "Point at the thing you want changed, and Clipframes tells Claude Code or Codex exactly which thing you mean. A free app for Mac and Windows.";

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

/// The hero has three layouts to compare (app/home.module.css): ?hero=a, b or c picks one, before
/// anything is drawn. With no choice the page is as it is built.
const hero = `try{var h=new URLSearchParams(location.search).get("hero");if(/^[abc]$/.test(h||""))document.documentElement.dataset.hero=h}catch(e){}`;

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{ __html: hero }} />
      </head>
      <body>
        {children}
        <Analytics />
      </body>
    </html>
  );
}
