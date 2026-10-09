import { LATEST_MAC, LATEST_WINDOWS } from "@/lib/site";

/// clipframes.app/download: the link to share. It starts the newest release's download for the
/// system the visitor is on, going by what their browser says it is.
export function GET(request: Request) {
  const windows = /Windows/i.test(request.headers.get("user-agent") ?? "");
  return new Response(null, { status: 307, headers: { Location: windows ? LATEST_WINDOWS : LATEST_MAC, "Cache-Control": "no-store", Vary: "User-Agent" } });
}
