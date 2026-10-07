import { describe, expect, it } from "vitest";
import { GET } from "./route";
import { LATEST_ZIP } from "@/lib/site";

describe("/download", () => {
  it("redirects to the newest release's zip, uncached", () => {
    const res = GET();
    expect(res.status).toBe(307);
    expect(res.headers.get("Location")).toBe(LATEST_ZIP);
    expect(res.headers.get("Cache-Control")).toBe("no-store");
  });
});
