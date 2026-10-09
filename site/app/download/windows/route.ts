import { LATEST_WINDOWS } from "@/lib/site";

export function GET() {
  return new Response(null, { status: 307, headers: { Location: LATEST_WINDOWS, "Cache-Control": "no-store" } });
}
