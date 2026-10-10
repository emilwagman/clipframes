/// Anonymous counts of how the site is used (app/privacy lists every one of them).
///
/// Nothing here runs until NEXT_PUBLIC_POSTHOG_KEY is set when the site is built: without it
/// the library is never fetched and no request is made. With it, the library is fetched after
/// the page has loaded, keeps nothing in the browser (no cookies, no local storage), records
/// no sessions, captures no clicks by itself, and makes no person profile.

type Props = Record<string, string>;

export type SiteEvent = "$pageview" | "download_clicked" | "link_copied" | "demo_started" | "demo_finished" | "reference_shown" | "reference_copied" | "recording_played";

export const HOST = "https://eu.i.posthog.com";

/// Everything an event is allowed to carry: the site's own four words, and the few things
/// the privacy page names. The library adds more by itself (the page's title and full address,
/// screen size, referrer, language, time zone); those are taken off again.
const KEPT = new Set(["os", "place", "demo", "by", "token", "distinct_id", "$pathname", "$os", "$browser", "$device_type", "$lib", "$lib_version", "$geoip_disable", "$process_person_profile"]);

interface Client {
  capture(event: string, props?: Props): unknown;
}

let client: Promise<Client> | null = null;

function load(key: string): Promise<Client> {
  client ??= import("posthog-js").then(({ default: posthog }) => {
    posthog.init(key, {
      api_host: HOST,
      persistence: "memory",
      person_profiles: "never",
      autocapture: false,
      rageclick: false,
      capture_pageview: false,
      capture_pageleave: false,
      capture_exceptions: false,
      capture_performance: false,
      capture_heatmaps: false,
      capture_dead_clicks: false,
      disable_session_recording: true,
      disable_surveys: true,
      disable_product_tours: true,
      disable_conversations: true,
      disable_web_experiments: true,
      disable_external_dependency_loading: true,
      advanced_disable_flags: true,
      save_referrer: false,
      save_campaign_params: false,
      before_send: (event) => {
        if (!event) return event;
        // No location is looked up from the address the event came from.
        const props: Record<string, unknown> = { $geoip_disable: true };
        for (const [name, value] of Object.entries(event.properties ?? {})) if (KEPT.has(name)) props[name] = value;
        return { ...event, properties: props, $set: undefined, $set_once: undefined };
      },
    });
    return posthog;
  });
  return client;
}

/// Counts one thing. Does nothing at all when the site was built without a key.
export function track(event: SiteEvent, props?: Props): void {
  const key = process.env.NEXT_PUBLIC_POSTHOG_KEY;
  if (!key || typeof window === "undefined") return;
  void load(key).then((posthog) => posthog.capture(event, props), () => {});
}

export const analyticsOn = (): boolean => Boolean(process.env.NEXT_PUBLIC_POSTHOG_KEY);
