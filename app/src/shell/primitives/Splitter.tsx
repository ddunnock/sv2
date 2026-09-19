// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * A draggable divider that sets a panel's size (WAI-ARIA APG, "Window
 * Splitter").
 *
 * It is a focusable `separator` that states its value, minimum and maximum, so
 * the keyboard and a screen reader can operate it as well as a pointer can:
 * arrow keys step it, Home and End jump to the limits. It sizes one panel —
 * the one it names with `aria-controls` — and the value is that panel's size
 * in CSS pixels.
 *
 * CONTROLLED. The width is a preference (`contract/preferences.ts`) and the
 * shell owns it. This reports where the user put the divider; clamping to the
 * limits happens here, so the caller never receives a value outside them.
 *
 * `side` is where the panel sits relative to the splitter. The navigator is at
 * the start and grows as the splitter moves right; the sidebar is at the end
 * and grows as it moves left.
 */

import type { KeyboardEvent, PointerEvent } from "react";

/** Props for `Splitter`. */
export type SplitterProps = Readonly<{
  /** Names the splitter by what it resizes: "Resize navigator". */
  label: string;
  /** The id of the panel it resizes. */
  controls: string;
  side: "start" | "end";
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}>;

/** How far one arrow press moves the divider, in CSS pixels. */
const STEP = 16;

/** A vertical divider between side-by-side panels. */
export function Splitter(props: SplitterProps): React.JSX.Element {
  const { label, controls, value, min, max } = props;
  const change = (next: number): void => {
    props.onChange(clamp(next, { min, max }));
  };
  // Moving the pointer right grows a start-side panel and shrinks an end-side one.
  const direction = props.side === "start" ? 1 : -1;

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>): void => {
    const next = keyValue(event.key, { value, min, max, direction });
    if (next !== null) {
      event.preventDefault();
      change(next);
    }
  };

  const onPointerDown = (event: PointerEvent<HTMLDivElement>): void => {
    const origin = { x: event.clientX, value };
    const target = event.currentTarget;
    target.setPointerCapture(event.pointerId);
    const onMove = (move: globalThis.PointerEvent): void => {
      change(origin.value + direction * (move.clientX - origin.x));
    };
    const onUp = (): void => {
      target.removeEventListener("pointermove", onMove);
      target.removeEventListener("pointerup", onUp);
    };
    target.addEventListener("pointermove", onMove);
    target.addEventListener("pointerup", onUp);
  };

  return (
    // biome-ignore lint/a11y/useSemanticElements: a window splitter has no native element; the APG pattern is a focusable role="separator", which <hr> cannot be.
    <div
      role="separator"
      aria-label={label}
      aria-controls={controls}
      aria-orientation="vertical"
      aria-valuenow={Math.round(value)}
      aria-valuemin={min}
      aria-valuemax={max}
      tabIndex={0}
      onKeyDown={onKeyDown}
      onPointerDown={onPointerDown}
      className="w-1 shrink-0 cursor-col-resize bg-line outline-none hover:bg-accent focus-visible:bg-accent"
    />
  );
}

/** `value` held within `[min, max]`. */
function clamp(value: number, limits: Readonly<{ min: number; max: number }>): number {
  return Math.min(Math.max(value, limits.min), limits.max);
}

/** Where a key puts the divider, or null for a key it does not handle. */
function keyValue(
  key: string,
  at: Readonly<{ value: number; min: number; max: number; direction: 1 | -1 }>,
): number | null {
  switch (key) {
    case "ArrowRight":
      return at.value + at.direction * STEP;
    case "ArrowLeft":
      return at.value - at.direction * STEP;
    case "Home":
      return at.min;
    case "End":
      return at.max;
    default:
      return null;
  }
}
