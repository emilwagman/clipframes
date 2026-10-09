import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const init = vi.fn();
const capture = vi.fn();
let loaded = 0;
vi.mock("posthog-js", () => {
  loaded += 1;
  return { default: { init, capture } };
});

const settle = () => new Promise((resolve) => setTimeout(resolve, 20));

describe("site analytics", () => {
  const fetchSpy = vi.fn();
  beforeEach(() => {
    vi.resetModules();
    init.mockClear();
    capture.mockClear();
    fetchSpy.mockClear();
    loaded = 0;
    vi.stubGlobal("window", {});
    vi.stubGlobal("fetch", fetchSpy);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
  });

  it("does nothing without a key: the library is not loaded and no request is made", async () => {
    vi.stubEnv("NEXT_PUBLIC_POSTHOG_KEY", "");
    const { track, analyticsOn } = await import("./analytics");
    track("$pageview");
    track("download_clicked", { os: "mac", place: "hero" });
    await settle();
    expect(analyticsOn()).toBe(false);
    expect(loaded).toBe(0);
    expect(init).not.toHaveBeenCalled();
    expect(capture).not.toHaveBeenCalled();
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("with a key, loads the library once and keeps nothing in the browser", async () => {
    vi.stubEnv("NEXT_PUBLIC_POSTHOG_KEY", "phc_test");
    const { track, HOST } = await import("./analytics");
    track("$pageview");
    track("demo_started", { demo: "hero" });
    await settle();
    expect(loaded).toBe(1);
    expect(init).toHaveBeenCalledTimes(1);
    const [key, config] = init.mock.calls[0];
    expect(key).toBe("phc_test");
    expect(HOST).toBe("https://eu.i.posthog.com");
    expect(config).toMatchObject({ api_host: HOST, persistence: "memory", person_profiles: "never", autocapture: false, disable_session_recording: true, capture_pageview: false, advanced_disable_flags: true, disable_external_dependency_loading: true });
    expect(capture.mock.calls).toEqual([["$pageview", undefined], ["demo_started", { demo: "hero" }]]);
  });

  it("takes off what the library adds by itself and asks for no location lookup", async () => {
    vi.stubEnv("NEXT_PUBLIC_POSTHOG_KEY", "phc_test");
    const { track } = await import("./analytics");
    track("$pageview");
    await settle();
    const before = init.mock.calls[0][1].before_send;
    const sent = before({ event: "download_clicked", properties: { os: "mac", place: "hero", $pathname: "/", $referrer: "https://example.com", $screen_width: 1280, $current_url: "https://clipframes.app/?x=1", title: "Clipframes", distinct_id: "abc" }, $set: { a: 1 } });
    expect(sent.properties).toEqual({ $geoip_disable: true, os: "mac", place: "hero", $pathname: "/", distinct_id: "abc" });
    expect(sent.$set).toBeUndefined();
  });
});
