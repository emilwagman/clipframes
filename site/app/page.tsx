import { HomePage } from "./HomePage";
import { starCount } from "@/lib/github";

export default async function Home() {
  return <HomePage stars={await starCount(3600)} />;
}
