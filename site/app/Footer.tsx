import { MAKER_URL, PAGES, REPO_URL } from "@/lib/site";
import s from "./home.module.css";

export default function Footer() {
  return (
    <footer className={s.footer}>
      <span>Made by <a href={MAKER_URL}>Emil Wagman</a></span>
      <span className={s.links}>
        {PAGES.map((page) => page.href && <a key={page.name} href={page.href}>{page.name}</a>)}
        <a href={REPO_URL}>Source on GitHub</a>
      </span>
    </footer>
  );
}
