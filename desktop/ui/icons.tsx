// Icons drawn as one stroked path each, in the colour of the text around them.

const PATHS = {
  // A frame with a pointer in its corner.
  element: "M4 3.5h9A1.5 1.5 0 0 1 14.5 5v3M4 3.5A1.5 1.5 0 0 0 2.5 5v7A1.5 1.5 0 0 0 4 13.5h4M10.5 9.5l7 2.6-3 1.2-1.2 3z",
  // The four corners of a dragged area.
  area: "M3 7V4.5A1.5 1.5 0 0 1 4.5 3H7M13 3h2.5A1.5 1.5 0 0 1 17 4.5V7M17 13v2.5a1.5 1.5 0 0 1-1.5 1.5H13M7 17H4.5A1.5 1.5 0 0 1 3 15.5V13",
  // A screen with a record dot.
  clip: "M4 4h12a1.5 1.5 0 0 1 1.5 1.5v9A1.5 1.5 0 0 1 16 16H4a1.5 1.5 0 0 1-1.5-1.5v-9A1.5 1.5 0 0 1 4 4zM10 7.6a2.4 2.4 0 1 0 0 4.8 2.4 2.4 0 0 0 0-4.8z",
  // A clock.
  history: "M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM10 6v4.2l2.8 1.6",
  // A pin: stays here.
  pin: "M7.5 3h5l-.7 5 2.7 2.5v1.5h-9v-1.5L8.2 8zM10 12v5",
  close: "M5 5l10 10M15 5L5 15",
  copy: "M7 7V4.5A1.5 1.5 0 0 1 8.5 3h7A1.5 1.5 0 0 1 17 4.5v7a1.5 1.5 0 0 1-1.5 1.5H13M4.5 7h7A1.5 1.5 0 0 1 13 8.5v7a1.5 1.5 0 0 1-1.5 1.5h-7A1.5 1.5 0 0 1 3 15.5v-7A1.5 1.5 0 0 1 4.5 7z",
  folder: "M3 6.5A1.5 1.5 0 0 1 4.5 5h3l1.5 2h6.5A1.5 1.5 0 0 1 17 8.5v6a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 3 14.5z",
  trash: "M4 6h12M8 6V4h4v2M6 6l.7 10h6.6L14 6",
} as const;

export type IconName = keyof typeof PATHS;

export function Icon({ name }: { name: IconName }) {
  return (
    <svg className="icon" viewBox="0 0 20 20" aria-hidden="true">
      <path d={PATHS[name]} />
    </svg>
  );
}
