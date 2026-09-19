// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The shell's icons, drawn inline and always decorative.
 *
 * An icon here never carries meaning on its own: the control it sits in has
 * the accessible name. So every `<svg>` is `aria-hidden`, which is what keeps
 * Biome's `a11y/noSvgWithoutTitle` satisfied honestly — the mockup trips it
 * about 25 times by leaving bare `<svg>`s for a screen reader to stumble over.
 *
 * The paths are 16-unit line drawings in the mockup's style: 1.3 stroke,
 * `currentColor`, so an icon takes its colour from the control around it.
 */

/** Every icon the shell draws. Adding one is a new key and a new path. */
export type IconName =
  | "files"
  | "search"
  | "validate"
  | "branch"
  | "settings"
  | "sidebar"
  | "navigator"
  | "window"
  | "close"
  | "chevron-right"
  | "chevron-down";

const PATHS: Readonly<Record<IconName, string>> = {
  files: "M3 2.5h6l4 4v7H3z M9 2.5v4h4",
  search: "M7 12A5 5 0 1 0 7 2a5 5 0 0 0 0 10z M10.5 10.5 14 14",
  validate: "M3 8.5 6.5 12 13 4",
  branch: "M5 3v10 M11 3v3a3 3 0 0 1-3 3H5",
  settings: "M8 10.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z M8 1.5v2 M8 12.5v2 M1.5 8h2 M12.5 8h2",
  sidebar: "M2.5 3h11v10h-11z M10 3v10",
  navigator: "M2.5 3h11v10h-11z M6 3v10",
  window: "M2.5 4h9v9h-9z M5 4V2.5h8.5V11H11.5",
  close: "M4 4l8 8 M12 4l-8 8",
  "chevron-right": "M6 3.5 10.5 8 6 12.5",
  "chevron-down": "M3.5 6 8 10.5 12.5 6",
};

/** Props for `Icon`. */
export type IconProps = Readonly<{ name: IconName }>;

/** A decorative 16-unit icon, hidden from assistive technology. */
export function Icon({ name }: IconProps): React.JSX.Element {
  return (
    <svg
      aria-hidden="true"
      focusable="false"
      viewBox="0 0 16 16"
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.3"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d={PATHS[name]} />
    </svg>
  );
}
