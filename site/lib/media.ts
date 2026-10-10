/// The recordings of the app the site shows (public/recordings), with the size each was made at.
/// The preview build puts the files themselves here in place of their addresses (scripts/build-preview.mjs).

export interface Media {
  src: string;
  poster: string;
  width: number;
  height: number;
  seconds: number;
}

export const RECORDINGS: Record<"desktop" | "comment" | "paste" | "native", Media> = {
  desktop: { src: "/recordings/desktop.mp4", poster: "/recordings/desktop.jpg", width: 1920, height: 1044, seconds: 31 },
  comment: { src: "/recordings/comment.mp4", poster: "/recordings/comment.jpg", width: 1280, height: 720, seconds: 9 },
  /// The app picking in Focus, a native Windows app, and pasting into Claude Code.
  native: { src: "/recordings/native.mp4", poster: "/recordings/native.jpg", width: 1920, height: 1044, seconds: 26 },
  paste: { src: "/recordings/paste.mp4", poster: "/recordings/paste.jpg", width: 1280, height: 478, seconds: 6 },
};
