import { describe, expect, it } from "vitest";
import { formatStars } from "./github";

describe("formatStars", () => {
  it("shows small counts as they are", () => {
    expect(formatStars(0)).toBe("0");
    expect(formatStars(999)).toBe("999");
  });
  it("shortens thousands", () => {
    expect(formatStars(1000)).toBe("1k");
    expect(formatStars(1234)).toBe("1.2k");
    expect(formatStars(9960)).toBe("10k");
    expect(formatStars(12500)).toBe("13k");
  });
});
