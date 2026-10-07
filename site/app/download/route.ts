import { LATEST_ZIP } from "@/lib/site";

/// clipframes.vercel.app/download: the link to share. It always starts the newest release's download.
export function GET() {
  return new Response(null, { status: 307, headers: { Location: LATEST_ZIP, "Cache-Control": "no-store" } });
}
