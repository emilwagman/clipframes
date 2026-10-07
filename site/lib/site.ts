/// The facts every part of the site shares.

export const SITE_URL = "https://clipframes.vercel.app";
export const SITE_NAME = "Clipframes";
export const REPO_URL = "https://github.com/emilwagman/clipframes";
export const MAKER_URL = "https://emilwagman.com";

/// The newest release's zip. Each release also uploads its zip under this stable name (app/release.sh).
export const LATEST_ZIP = `${REPO_URL}/releases/latest/download/Clipframes.zip`;

/// The site's own download address, which redirects to the newest zip (app/download/route.ts).
export const DOWNLOAD_PATH = "/download";

/// Shown beside the download buttons. Bump with each release.
export const VERSION = "0.2";
export const REQUIREMENTS = "macOS 15 or later · Apple Silicon and Intel";
