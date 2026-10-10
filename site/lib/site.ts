/// The facts every part of the site shares.

export const SITE_URL = "https://clipframes.app";
export const SITE_NAME = "Clipframes";
export const REPO_URL = "https://github.com/emilwagman/clipframes";
export const MAKER_URL = "https://emilwagman.com";

/// The newest release's files. Each release uploads them under these stable names (desktop/release.sh).
export const LATEST_MAC = `${REPO_URL}/releases/latest/download/Clipframes.zip`;
export const LATEST_WINDOWS = `${REPO_URL}/releases/latest/download/Clipframes-setup.exe`;

/// The site's own download addresses, which redirect to the newest files (app/download).
/// Picks the file for the visitor's system; the header's one button uses it.
export const DOWNLOAD_PATH = "/download";
export const DOWNLOAD_MAC = "/download/mac";
export const DOWNLOAD_WINDOWS = "/download/windows";

/// Shown beside the download buttons. Bump with each release.
export const VERSION = "0.3";
export const REQUIREMENTS = "macOS 12 or later · Windows 10 or 11";

/// The pages that answer what a visitor asks, in the order they ask. One without an address is
/// not built yet and is shown as a word only.
export const PAGES: { name: string; href?: string }[] = [
  { name: "How it works" },
  { name: "Compare", href: "/compare" },
  { name: "Install" },
  { name: "Privacy", href: "/privacy" },
];
