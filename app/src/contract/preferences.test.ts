// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { PREFERENCES, THEMES } from "./preferences";

describe("storage keys", () => {
  const keys = Object.values(PREFERENCES).map((entry) => entry.storageKey);

  test("every preference has its own key, so each fails alone", () => {
    expect(new Set(keys).size).toBe(keys.length);
  });

  test.each(keys)("%s is namespaced and versioned", (key) => {
    expect(key).toMatch(/^sv2\.[a-z-]+\.v[0-9]+$/);
  });
});

describe("theme", () => {
  const { schema } = PREFERENCES.theme;

  test.each([...THEMES])("%s is a theme", (value) => {
    expect(schema.safeParse(value).success).toBe(true);
  });

  test.each(["Dark", "high-contrast", "", null])("%s is not a theme", (value) => {
    expect(schema.safeParse(value).success).toBe(false);
  });
});

describe("navigator", () => {
  const { schema } = PREFERENCES.navigator;

  test.each([
    ["hidden", { kind: "hidden" }],
    ["open on files", { kind: "open", mode: "files" }],
    ["open on elements", { kind: "open", mode: "elements" }],
  ])("%s is a navigator state", (_name, value) => {
    expect(schema.safeParse(value).success).toBe(true);
  });

  test.each([
    // §4.5: the two booleans this union replaces.
    ["a visibility flag beside a mode", { visible: false, mode: "files" }],
    ["open with no mode", { kind: "open" }],
    ["hidden with a mode", { kind: "hidden", mode: "files" }],
    ["a mode outside IX-01", { kind: "open", mode: "search" }],
  ])("%s is not a navigator state", (_name, value) => {
    expect(schema.safeParse(value).success).toBe(false);
  });
});

describe("sidebar", () => {
  const { schema } = PREFERENCES.sidebar;

  test.each([
    ["open on the specification", { kind: "open", tab: "specification" }],
    ["collapsed, remembering the source tab (IX-03)", { kind: "collapsed", tab: "source" }],
  ])("%s is a sidebar state", (_name, value) => {
    expect(schema.safeParse(value).success).toBe(true);
  });

  test.each([
    ["collapsed with no tab to return to", { kind: "collapsed" }],
    ["a tab the sidebar does not have", { kind: "open", tab: "diagnostics" }],
  ])("%s is not a sidebar state", (_name, value) => {
    expect(schema.safeParse(value).success).toBe(false);
  });
});

describe("panel widths", () => {
  test.each([
    ["navigatorWidth", PREFERENCES.navigatorWidth.schema],
    ["sidebarWidth", PREFERENCES.sidebarWidth.schema],
  ])(
    "%s takes the mockup's widths, and rejects what storage should never hold",
    (_name, schema) => {
      expect(schema.safeParse(280).success).toBe(true);
      expect(schema.safeParse(340).success).toBe(true);
      for (const bad of [0, -280, 280.5, "280", Number.NaN, 0x1_0000]) {
        expect(schema.safeParse(bad).success).toBe(false);
      }
    },
  );
});
