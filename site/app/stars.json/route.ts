import { starCount } from "@/lib/github";

/// clipframes.app/stars.json: the current star count, at most five minutes old. The header reads
/// it after the page loads, so the number stays fresh without a visitor's browser calling GitHub.
export const revalidate = 300;

export async function GET() {
  const stars = await starCount(300);
  return Response.json({ stars }, { headers: { "Cache-Control": "public, max-age=60, s-maxage=300" } });
}
