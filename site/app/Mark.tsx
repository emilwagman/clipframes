/// The Clipframes mark in the text colour: a frame with its corner clipped off, the piece lifted away.
/// Same geometry as public/mark.svg and the app (app/Sources/Logo.swift).
export default function Mark({ size = 24 }: { size?: number }) {
  return (
    <svg viewBox="0 0 114 114" width={size} height={size} fill="currentColor" aria-hidden="true">
      <path fillRule="evenodd" d="M4 10.2H76L104 38.2V110.2H4Z M21 27.2V93.2H87V27.2Z" />
      <path d="M83.6 4H110V30.4Z" />
    </svg>
  );
}
