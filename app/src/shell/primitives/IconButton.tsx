// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * A button that shows an icon and is named for assistive technology.
 *
 * THE NAME IS REQUIRED. An icon is not a name: without `label` a screen reader
 * announces "button" and nothing else, which is the defect the mockup has 128
 * times over. `label` becomes the accessible name and the tooltip.
 *
 * A DISABLED BUTTON SAYS WHY. The plan's rule for deferred features is a
 * disabled control whose title names the reason, never a control that silently
 * does nothing. So `availability` is a union and the disabled arm cannot be
 * built without a reason (§8.2 rule 2).
 *
 * DISABLED, BUT STILL REACHABLE. It uses `aria-disabled` rather than the
 * `disabled` attribute, which would drop it from the tab order — and with it
 * the only way a keyboard user could hear the reason. A press is ignored in
 * the handler instead.
 */

import type { ReactNode } from "react";

/** Whether the button acts, and if not, why not. */
export type Availability =
  | Readonly<{ kind: "enabled" }>
  | Readonly<{ kind: "disabled"; reason: string }>;

/** A convenience for the common case. */
export const ENABLED: Availability = { kind: "enabled" };

/** Props for `IconButton`. */
export type IconButtonProps = Readonly<{
  /** The accessible name, and the tooltip. */
  label: string;
  /** The icon, which must be decorative: see `Icon`. */
  icon: ReactNode;
  availability: Availability;
  onPress: () => void;
  /** For a toggle: whether it is on. Absent for an ordinary action. */
  pressed?: boolean;
  /** Roving focus, set by a container such as `Toolbar`. Absent means reachable by Tab. */
  tabIndex?: 0 | -1;
}>;

/** A named, icon-only button. */
export function IconButton({
  label,
  icon,
  availability,
  onPress,
  pressed,
  tabIndex,
}: IconButtonProps): React.JSX.Element {
  const disabled = availability.kind === "disabled";
  return (
    <button
      type="button"
      aria-label={label}
      aria-disabled={disabled ? true : undefined}
      aria-pressed={pressed}
      title={disabled ? `${label} — ${availability.reason}` : label}
      tabIndex={tabIndex}
      onClick={() => {
        if (!disabled) {
          onPress();
        }
      }}
      className="inline-flex size-control items-center justify-center rounded-wb text-muted hover:bg-hover hover:text-fg aria-disabled:cursor-not-allowed aria-disabled:opacity-50 aria-disabled:hover:bg-transparent aria-pressed:bg-accent-bg aria-pressed:text-accent"
    >
      {icon}
    </button>
  );
}
