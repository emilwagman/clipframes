import type { Metadata } from "next";
import { starCount } from "@/lib/github";
import { SITE_NAME } from "@/lib/site";
import { ComparePage } from "./ComparePage";

export const metadata: Metadata = {
  title: `Compare · ${SITE_NAME}`,
  description: "How Clipframes compares with a screenshot and with Agentation.",
};

export default async function Compare() {
  return <ComparePage stars={await starCount(3600)} />;
}
