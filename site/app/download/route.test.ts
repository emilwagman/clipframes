import { describe, expect, it } from "vitest";
import { GET } from "./route";
import { LATEST_MAC, LATEST_WINDOWS } from "@/lib/site";

const from = (agent: string) => GET(new Request("https://clipframes.app/download", { headers: { "user-agent": agent } }));

describe("/download", () => {
  it("sends a Mac to the Mac app", () => {
    const res = from("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Safari/605.1.15");
    expect(res.status).toBe(307);
    expect(res.headers.get("Location")).toBe(LATEST_MAC);
  });

  it("sends a Windows PC to the Windows installer", () => {
    const res = from("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/140.0 Safari/537.36");
    expect(res.headers.get("Location")).toBe(LATEST_WINDOWS);
  });

  it("is never cached, so it always points at the newest release", () => {
    expect(from("").headers.get("Cache-Control")).toBe("no-store");
  });
});
