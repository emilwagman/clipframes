import { MAKER_URL, REPO_URL } from "@/lib/site";
import s from "./home.module.css";

export default function Footer() {
  return (
    <footer className={s.footer}>
      <span>Made by <a href={MAKER_URL}>Emil Wagman</a></span>
      <span className={s.links}>
        <a href="/privacy">Privacy</a>
        <a href={REPO_URL}>Source on GitHub</a>
      </span>
    </footer>
  );
}
