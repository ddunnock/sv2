// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * A row or column of icon buttons, navigated as one control (WAI-ARIA APG,
 * "Toolbar").
 *
 * ONE TAB STOP. Tab enters the toolbar and Tab leaves it; the arrow keys move
 * between its buttons, and Home and End jump to the ends. Only the button that
 * last had focus is in the tab order — a roving `tabIndex` — so a keyboard user
 * crosses a twelve-button toolbar in one keystroke, not twelve.
 *
 * DATA, NOT CHILDREN. The toolbar owns the roving index, so it renders its own
 * buttons from `items`. Arbitrary children would leave it guessing which of
 * them are focusable, and a guess is how roving focus breaks.
 *
 * A disabled item keeps its place in the roving order (APG: disabled toolbar
 * items stay focusable), so its reason can be heard.
 */

import { type KeyboardEvent, useRef, useState } from "react";
import { Icon, type IconName } from "./Icon";
import { type Availability, IconButton } from "./IconButton";
import { type Orientation, rovingTarget } from "./roving";

/** One toolbar button. `id` is stable across renders; `label` is its accessible name. */
export type ToolbarItem = Readonly<{
  id: string;
  label: string;
  icon: IconName;
  availability: Availability;
  onPress: () => void;
  pressed?: boolean;
}>;

/** Props for `Toolbar`. */
export type ToolbarProps = Readonly<{
  /** Names the toolbar as a whole: "Activity", "View tools". */
  label: string;
  orientation: Orientation;
  items: readonly ToolbarItem[];
}>;

/** A named group of icon buttons with one tab stop. */
export function Toolbar({ label, orientation, items }: ToolbarProps): React.JSX.Element {
  const [active, setActive] = useState(0);
  const container = useRef<HTMLDivElement>(null);
  // Derived, not mirrored (§8.3 rule 1): an index past a shrunken list is clamped here.
  const current = Math.min(active, Math.max(items.length - 1, 0));

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>): void => {
    const next = rovingTarget(event.key, { current, count: items.length, orientation });
    if (next === null) {
      return;
    }
    event.preventDefault();
    setActive(next);
    container.current?.querySelectorAll("button")[next]?.focus();
  };

  return (
    <div
      ref={container}
      role="toolbar"
      aria-label={label}
      aria-orientation={orientation}
      onKeyDown={onKeyDown}
      className={`flex gap-0.5 ${orientation === "vertical" ? "flex-col" : "flex-row"}`}
    >
      {items.map((item, index) => (
        <IconButton
          key={item.id}
          label={item.label}
          icon={<Icon name={item.icon} />}
          availability={item.availability}
          onPress={() => {
            setActive(index);
            item.onPress();
          }}
          {...(item.pressed === undefined ? {} : { pressed: item.pressed })}
          tabIndex={index === current ? 0 : -1}
        />
      ))}
    </div>
  );
}
