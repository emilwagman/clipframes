/// The repository's star count, fetched server-side (no token) and cached. Anything that fails
/// returns null and the header simply shows "GitHub" without a number.

export const REPO = "emilwagman/clipframes";

const headers = { Accept: "application/vnd.github+json", "User-Agent": "clipframes-site" };

/// `revalidate`: how many seconds a fetched count may be reused.
export async function starCount(revalidate: number): Promise<number | null> {
  try {
    const r = await fetch(`https://api.github.com/repos/${REPO}`, { next: { revalidate }, headers });
    if (!r.ok) return null;
    const n = ((await r.json()) as { stargazers_count?: unknown }).stargazers_count;
    return typeof n === "number" && Number.isInteger(n) && n >= 0 ? n : null;
  } catch {
    return null;
  }
}

/// 1234 → "1.2k", so the header stays short.
export function formatStars(n: number): string {
  if (n < 1000) return String(n);
  const k = n / 1000;
  return (k < 10 ? k.toFixed(1).replace(/\.0$/, "") : Math.round(k).toString()) + "k";
}
