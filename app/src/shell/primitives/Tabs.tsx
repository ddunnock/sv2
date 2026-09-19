// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * A tab list and the panel of its selected tab (WAI-ARIA APG, "Tabs").
 *
 * THE MOCKUP HALF-APPLIES THIS, which is worse than not applying it: its tabs
 * carry `role="tab"` with no `tabpanel`, no `aria-controls` and no roving
 * `tabIndex`, so a screen reader announces a tab set that cannot be operated
 * as one. This is the whole pattern:
 *
 * - The list is one tab stop. Arrow keys move between tabs along the
 *   orientation, Home and End jump to the ends.
 * - Each tab names the panel it controls, and the panel names its tab.
 * - `activation` decides whether moving focus selects. `automatic` suits a
 *   cheap switch such as the sidebar's two tabs; `manual` suits view tabs,
 *   where selecting opens a view. Under manual activation Enter and Space
 *   select, which a native button already does, so there is no code for it.
 *
 * A LIST WITHOUT A PANEL. The collapsed sidebar (UI-09) is a vertical strip of
 * the same tabs with nothing expanded. `panel` is then null, and the tabs name
 * no panel rather than pointing at one that is not in the document.
 */

import { type KeyboardEvent, type ReactNode, useId, useRef, useState } from "react";

import { type Orientation, rovingTarget } from "./roving";

/** One tab. `label` is its accessible name and what it shows. */
export type TabItem = Readonly<{ id: string; label: string }>;

/** Props for `Tabs`. */
export type TabsProps = Readonly<{
  /** Names the tab list: "Sidebar", "Open views". */
  label: string;
  tabs: readonly TabItem[];
  selected: string;
  onSelect: (id: string) => void;
  activation: "automatic" | "manual";
  orientation: Orientation;
  /** The selected tab's content, or null for a list with nothing expanded. */
  panel: ReactNode | null;
}>;

/** A tab list with one tab stop, and the selected tab's panel. */
export function Tabs(props: TabsProps): React.JSX.Element {
  const { label, tabs, selected, orientation, panel } = props;
  const base = useId();
  const list = useRef<HTMLDivElement>(null);
  const selectedIndex = Math.max(
    tabs.findIndex((tab) => tab.id === selected),
    0,
  );
  // Where focus is within the list. Starts on the selected tab, which is where
  // the APG puts the tab stop.
  const [focused, setFocused] = useState<number | null>(null);
  const stop = focused === null || focused >= tabs.length ? selectedIndex : focused;

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>): void => {
    const next = rovingTarget(event.key, { current: stop, count: tabs.length, orientation });
    const tab = next === null ? undefined : tabs[next];
    if (next === null || tab === undefined) {
      return;
    }
    event.preventDefault();
    setFocused(next);
    list.current?.querySelectorAll<HTMLElement>('[role="tab"]')[next]?.focus();
    if (props.activation === "automatic") {
      props.onSelect(tab.id);
    }
  };

  const tabId = (id: string): string => `${base}-tab-${id}`;
  const panelId = `${base}-panel`;

  return (
    <div className={`flex ${orientation === "vertical" ? "flex-row" : "flex-col"} min-h-0`}>
      <div
        ref={list}
        role="tablist"
        aria-label={label}
        aria-orientation={orientation}
        onKeyDown={onKeyDown}
        className={`flex ${orientation === "vertical" ? "flex-col" : "flex-row"} border-line`}
      >
        {tabs.map((tab, index) => (
          <button
            key={tab.id}
            type="button"
            role="tab"
            id={tabId(tab.id)}
            aria-selected={tab.id === selected}
            aria-controls={panel === null ? undefined : panelId}
            tabIndex={index === stop ? 0 : -1}
            onClick={() => {
              setFocused(index);
              props.onSelect(tab.id);
            }}
            className="border-transparent border-b-2 px-3 text-muted text-ui hover:text-fg aria-selected:border-accent aria-selected:text-fg"
          >
            {tab.label}
          </button>
        ))}
      </div>
      {panel === null ? null : (
        <div
          role="tabpanel"
          id={panelId}
          aria-labelledby={tabId(selected)}
          className="min-h-0 flex-1"
        >
          {panel}
        </div>
      )}
    </div>
  );
}
