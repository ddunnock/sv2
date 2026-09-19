// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Type tests for the preference table (§11 rule 11).
 *
 * Never run; `tsc` checks them. The table's mapped type is what keeps every
 * preference stored and every stored value read as the right type, and that
 * is a property of the type, not of any value.
 */

import type { z } from "zod";

import type { PreferenceEntry, Preferences, Theme } from "./preferences";

type Table = { [K in keyof Preferences]: PreferenceEntry<Preferences[K]> };

declare const complete: Table;

/** A table missing a preference does not compile. */
// @ts-expect-error sidebarWidth has no entry
export const missing: Table = {
  theme: complete.theme,
  navigator: complete.navigator,
  sidebar: complete.sidebar,
  navigatorWidth: complete.navigatorWidth,
};

/** An entry whose schema reads the wrong type does not compile. */
export const wrongSchema: Table = {
  ...complete,
  // @ts-expect-error a width schema cannot read the theme
  theme: complete.navigatorWidth,
};

/** A read value has the preference's type, not unknown. */
declare const parsed: z.ZodSafeParseResult<Theme>;
export const theme: Theme | undefined = parsed.success ? parsed.data : undefined;
