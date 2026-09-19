// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Per-machine UI preferences, as `localStorage` holds them (§8.6).
 *
 * RUST COUNTERPART: none, and none is wanted. These never cross IPC; §8.6 puts
 * anything that should survive a clone of the repository in a file the Rust
 * side writes, and leaves only this machine's panel state here. The boundary
 * is still a boundary — an older build may have written what a newer one reads
 * (§4.1) — so every value is parsed, by the one `shell/` module §8.6 names.
 *
 * ONE KEY PER PREFERENCE, EACH PARSED ALONE. §4.3 rule 3 forbids a `.catch()`
 * default, so a single record holding every preference would lose all of them
 * to one bad field. Separate keys fail separately: a stored theme another build
 * wrote wrongly costs the theme, not the panel widths. The shell treats a
 * failed parse as "no preference saved" and falls back to its own default.
 *
 * A BREAKING SHAPE CHANGE BUMPS THE KEY. Each storage key ends in a version.
 * Changing a shape without changing the key would make every stored value fail
 * its parse — harmless, since that means "no preference", but it would also be
 * silent; a new key says in the code that the old values are abandoned.
 *
 * DEFAULTS ARE NOT HERE. The mockup's 280 px navigator and 340 px sidebar are
 * layout decisions the shell owns; this module says only what a stored value
 * may look like.
 *
 * NOT HERE YET: anything per workspace, such as open view tabs. `ViewId`s mean
 * something only within one workspace, and `localStorage` is per machine, so
 * those need a workspace key that no contract module has yet.
 */

import { z } from "zod";

/** The colour theme, or `system` to follow the platform's setting. */
export const THEMES = ["system", "light", "dark"] as const;

/** The colour theme. */
export type Theme = (typeof THEMES)[number];

const ThemeSchema: z.ZodType<Theme, unknown> = z.enum(THEMES);

/**
 * The navigator (UI-03): hidden, or open in one of IX-01's two modes.
 *
 * A union rather than a visibility flag beside a mode, per §4.5.
 */
export type NavigatorState =
  | Readonly<{ kind: "hidden" }>
  | Readonly<{ kind: "open"; mode: "files" | "elements" }>;

const NavigatorStateSchema: z.ZodType<NavigatorState, unknown> = z.discriminatedUnion("kind", [
  z.strictObject({ kind: z.literal("hidden") }),
  z.strictObject({ kind: z.literal("open"), mode: z.enum(["files", "elements"]) }),
]);

/**
 * The Specification sidebar (UI-08, UI-09).
 *
 * `collapsed` still carries `tab`, because IX-03 expands the sidebar back to the
 * tab it was showing.
 */
export type SidebarState = Readonly<{
  kind: "collapsed" | "open";
  tab: "specification" | "source";
}>;

const SidebarStateSchema: z.ZodType<SidebarState, unknown> = z.strictObject({
  kind: z.enum(["collapsed", "open"]),
  tab: z.enum(["specification", "source"]),
});

/**
 * A panel width, in CSS pixels.
 *
 * Only checked as a plausible stored value. Clamping it to the window is the
 * shell's job, since the window may be smaller now than when it was saved.
 */
const PanelWidthSchema: z.ZodType<number, unknown> = z.number().int().positive().max(0xffff);

/** Every preference, by name. */
export type Preferences = Readonly<{
  theme: Theme;
  navigator: NavigatorState;
  sidebar: SidebarState;
  navigatorWidth: number;
  sidebarWidth: number;
}>;

/** Where one preference is stored, and what a stored value must look like. */
export type PreferenceEntry<T> = Readonly<{ storageKey: string; schema: z.ZodType<T, unknown> }>;

/**
 * Every preference's storage key and schema.
 *
 * The mapped type means adding a field to `Preferences` without an entry here
 * does not compile, and neither does an entry whose schema reads the wrong type.
 */
export const PREFERENCES: Readonly<{ [K in keyof Preferences]: PreferenceEntry<Preferences[K]> }> =
  {
    theme: { storageKey: "sv2.theme.v1", schema: ThemeSchema },
    navigator: { storageKey: "sv2.navigator.v1", schema: NavigatorStateSchema },
    sidebar: { storageKey: "sv2.sidebar.v1", schema: SidebarStateSchema },
    navigatorWidth: { storageKey: "sv2.navigator-width.v1", schema: PanelWidthSchema },
    sidebarWidth: { storageKey: "sv2.sidebar-width.v1", schema: PanelWidthSchema },
  };
