// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Where a key moves focus in a one-dimensional roving group (WAI-ARIA APG,
 * "Keyboard Navigation Inside Components").
 *
 * `Toolbar` and `Tabs` share this exactly, and so does the up-and-down half of
 * `Tree`: the arrows along the group's axis step and wrap, Home and End jump to
 * the ends, and every other key is the caller's. One definition, so the three
 * cannot drift into three slightly different keyboards.
 */

/** Which arrows step the group. */
export type Orientation = "horizontal" | "vertical";

const STEP_KEYS: Readonly<Record<Orientation, readonly [string, string]>> = {
  horizontal: ["ArrowLeft", "ArrowRight"],
  vertical: ["ArrowUp", "ArrowDown"],
};

/** Where focus is, how many there are, and which axis the arrows run along. */
export type RovingPosition = Readonly<{ current: number; count: number; orientation: Orientation }>;

/** The index `key` moves focus to, or null for a key the group does not handle. */
export function rovingTarget(key: string, where: RovingPosition): number | null {
  const [back, forward] = STEP_KEYS[where.orientation];
  const last = where.count - 1;
  if (where.count === 0) {
    return null;
  }
  switch (key) {
    case back:
      return where.current === 0 ? last : where.current - 1;
    case forward:
      return where.current === last ? 0 : where.current + 1;
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return null;
  }
}
