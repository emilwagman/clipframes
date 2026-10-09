import { DOWNLOAD_MAC, DOWNLOAD_WINDOWS } from "@/lib/site";
import s from "./home.module.css";

export function DownloadIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M12 4v12M6.5 10.5L12 16l5.5-5.5M5 20h14" />
    </svg>
  );
}

/// The two download buttons, one per system.
export function DownloadButtons() {
  return (
    <>
      <a className={`${s.pill} ${s.big}`} href={DOWNLOAD_MAC}><DownloadIcon />Download for Mac</a>
      <a className={`${s.pill} ${s.big}`} href={DOWNLOAD_WINDOWS}><DownloadIcon />Download for Windows</a>
    </>
  );
}
