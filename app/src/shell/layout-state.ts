// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The window's layout: what is shown, and how it changes (IX-02, IX-03, IX-09).
 *
 * ONE WINDOW, NOT FIVE SCREENS. The mockup's SCR-01 to SCR-04 are the same
 * window with a handful of flags varied; `workbench.js` proves it. So the
 * layout is one state value, and every screen is a value of it.
 *
 * PURE. A reducer over a union of actions, so every transition is a function
 * call in a test rather than a sequence of clicks.
 *
 * FOCUS MODE IS EXACTLY REVERSIBLE (IX-09). Entering it records the navigator
 * and sidebar as they were; leaving it restores that record, whatever it was.
 * An explicit toggle while focused leaves focus mode and applies to the
 * current layout, and the record is discarded — otherwise F11 would later undo
 * a choice the user had just made.
 */

import type { NavigatorState, SidebarState } from "@/contract/preferences";
import { assertNever } from "@/model/assert-never";

/** Normal, or focused with what to restore on the way out. */
export type LayoutMode =
  | Readonly<{ kind: "normal" }>
  | Readonly<{
      kind: "focus";
      restore: Readonly<{ navigator: NavigatorState; sidebar: SidebarState }>;
    }>;

/** The whole layout. */
export type LayoutState = Readonly<{
  navigator: NavigatorState;
  sidebar: SidebarState;
  mode: LayoutMode;
}>;

/** Everything that changes the layout. */
export type LayoutAction =
  | Readonly<{ kind: "toggle-navigator" }>
  | Readonly<{ kind: "show-navigator-mode"; mode: NavigatorState["mode"] }>
  | Readonly<{ kind: "toggle-sidebar" }>
  | Readonly<{ kind: "show-sidebar-tab"; tab: SidebarState["tab"] }>
  | Readonly<{ kind: "toggle-focus" }>;

/** The mockup's SCR-01: navigator open on Files, sidebar open on Specification. */
export const INITIAL_LAYOUT: LayoutState = {
  navigator: { kind: "open", mode: "files" },
  sidebar: { kind: "open", tab: "specification" },
  mode: { kind: "normal" },
};

const NORMAL: LayoutMode = { kind: "normal" };

/** The layout after `action`. */
export function layoutReducer(state: LayoutState, action: LayoutAction): LayoutState {
  switch (action.kind) {
    case "toggle-navigator": {
      const kind = state.navigator.kind === "open" ? "hidden" : "open";
      return { ...state, mode: NORMAL, navigator: { ...state.navigator, kind } };
    }
    case "show-navigator-mode":
      return { ...state, mode: NORMAL, navigator: { kind: "open", mode: action.mode } };
    case "toggle-sidebar": {
      const kind = state.sidebar.kind === "open" ? "collapsed" : "open";
      return { ...state, mode: NORMAL, sidebar: { ...state.sidebar, kind } };
    }
    case "show-sidebar-tab":
      // IX-03: choosing a tab on the collapsed strip expands to that tab.
      return { ...state, mode: NORMAL, sidebar: { kind: "open", tab: action.tab } };
    case "toggle-focus":
      return toggleFocus(state);
    default:
      return assertNever(action);
  }
}

/** Enter focus mode recording the layout, or leave it restoring the record. */
function toggleFocus(state: LayoutState): LayoutState {
  if (state.mode.kind === "focus") {
    return { ...state.mode.restore, mode: NORMAL };
  }
  return {
    navigator: { ...state.navigator, kind: "hidden" },
    sidebar: { ...state.sidebar, kind: "collapsed" },
    mode: { kind: "focus", restore: { navigator: state.navigator, sidebar: state.sidebar } },
  };
}

/** The keys a shortcut reads, so this stays testable without a DOM event. */
export type KeyPress = Readonly<{
  code: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}>;

/**
 * The action a key press asks for, if it is one of IX-09's shortcuts:
 * Ctrl+B the navigator, Ctrl+Alt+B the sidebar, F11 focus mode.
 *
 * Matched on `code`, not `key`, because Alt changes the character a key
 * produces on some layouts — Option+B on a Mac types "∫" — and the shortcut is
 * a position on the keyboard, not a letter. Cmd stands in for Ctrl on a Mac.
 */
export function shortcutAction(press: KeyPress): LayoutAction | null {
  const mod = press.ctrlKey || press.metaKey;
  if (press.code === "F11" && !mod && !press.altKey && !press.shiftKey) {
    return { kind: "toggle-focus" };
  }
  if (press.code !== "KeyB" || !mod || press.shiftKey) {
    return null;
  }
  return press.altKey ? { kind: "toggle-sidebar" } : { kind: "toggle-navigator" };
}
