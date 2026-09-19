// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import {
  INITIAL_LAYOUT,
  type KeyPress,
  type LayoutAction,
  type LayoutState,
  layoutReducer,
  shortcutAction,
} from "./layout-state";

const run = (state: LayoutState, ...actions: LayoutAction[]): LayoutState =>
  actions.reduce(layoutReducer, state);

describe("layoutReducer", () => {
  test("toggling the navigator twice returns it to the mode it was in", () => {
    const elements = run(INITIAL_LAYOUT, { kind: "show-navigator-mode", mode: "elements" });
    const back = run(elements, { kind: "toggle-navigator" }, { kind: "toggle-navigator" });
    expect(back.navigator).toEqual({ kind: "open", mode: "elements" });
  });

  test("choosing a navigator mode opens a hidden navigator on it", () => {
    const hidden = run(INITIAL_LAYOUT, { kind: "toggle-navigator" });
    expect(run(hidden, { kind: "show-navigator-mode", mode: "elements" }).navigator).toEqual({
      kind: "open",
      mode: "elements",
    });
  });

  test("a collapsed sidebar expands back to the tab it was showing (IX-03)", () => {
    const source = run(INITIAL_LAYOUT, { kind: "show-sidebar-tab", tab: "source" });
    const back = run(source, { kind: "toggle-sidebar" }, { kind: "toggle-sidebar" });
    expect(back.sidebar).toEqual({ kind: "open", tab: "source" });
  });

  test("choosing a tab on the collapsed strip expands to that tab (IX-03)", () => {
    const collapsed = run(INITIAL_LAYOUT, { kind: "toggle-sidebar" });
    expect(run(collapsed, { kind: "show-sidebar-tab", tab: "source" }).sidebar).toEqual({
      kind: "open",
      tab: "source",
    });
  });

  describe("focus mode (IX-09)", () => {
    test("hides the navigator and collapses the sidebar, keeping their modes", () => {
      const focused = run(INITIAL_LAYOUT, { kind: "toggle-focus" });
      expect(focused.navigator).toEqual({ kind: "hidden", mode: "files" });
      expect(focused.sidebar).toEqual({ kind: "collapsed", tab: "specification" });
      expect(focused.mode.kind).toBe("focus");
    });

    test("is exactly reversible, whatever the layout was", () => {
      const custom = run(
        INITIAL_LAYOUT,
        { kind: "toggle-navigator" },
        { kind: "show-sidebar-tab", tab: "source" },
      );
      expect(run(custom, { kind: "toggle-focus" }, { kind: "toggle-focus" })).toEqual(custom);
    });

    test("an explicit toggle while focused leaves focus mode and discards the record", () => {
      const focused = run(INITIAL_LAYOUT, { kind: "toggle-focus" });
      const shown = run(focused, { kind: "toggle-navigator" });
      expect(shown.mode).toEqual({ kind: "normal" });
      expect(shown.navigator.kind).toBe("open");
      // The sidebar stays as focus mode left it, not as it was before focus.
      expect(shown.sidebar.kind).toBe("collapsed");
      // So F11 now enters focus mode afresh rather than undoing the toggle.
      expect(run(shown, { kind: "toggle-focus" }).mode.kind).toBe("focus");
    });
  });
});

describe("shortcutAction", () => {
  const press = (over: Partial<KeyPress>): KeyPress => ({
    code: "KeyB",
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    shiftKey: false,
    ...over,
  });

  test.each<[string, KeyPress, LayoutAction | null]>([
    ["Ctrl+B toggles the navigator", press({ ctrlKey: true }), { kind: "toggle-navigator" }],
    ["Cmd+B does too, on a Mac", press({ metaKey: true }), { kind: "toggle-navigator" }],
    [
      "Ctrl+Alt+B toggles the sidebar",
      press({ ctrlKey: true, altKey: true }),
      { kind: "toggle-sidebar" },
    ],
    ["F11 toggles focus mode", press({ code: "F11" }), { kind: "toggle-focus" }],
    ["B alone is typing, not a shortcut", press({}), null],
    ["Ctrl+Shift+B is not one of IX-09's", press({ ctrlKey: true, shiftKey: true }), null],
    ["Ctrl+F11 is not F11", press({ code: "F11", ctrlKey: true }), null],
    ["Ctrl+N is not a layout shortcut", press({ code: "KeyN", ctrlKey: true }), null],
  ])("%s", (_name, key, action) => {
    expect(shortcutAction(key)).toEqual(action);
  });
});
