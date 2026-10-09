import { LATEST_MAC } from "@/lib/site";

export function GET() {
  return new Response(null, { status: 307, headers: { Location: LATEST_MAC, "Cache-Control": "no-store" } });
}
