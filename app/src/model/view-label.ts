// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * How a view is named on screen: its kind's name and badge, and its caption.
 *
 * Presentation, derived here so it never crosses the wire (`contract/view.ts`
 * carries a kind and what the view exposes, nothing more). The mockup gives
 * five badges — GV, IV, AF, ST, GR — and the other three follow the same rule:
 * two letters from the definition's name.
 *
 * THE CAPTION IS WHAT THE VIEW EXPOSES. The mockup names every view by kind
 * and exposed element — "GV ThermalControl" — never by the view's own name,
 * because the exposed element is what a reader is looking for. A view that
 * exposes nothing falls back to its own name.
 */

import type { ElementRef } from "@/contract/element";
import type { ViewKind, ViewSummary } from "@/contract/view";

import { assertNever } from "./assert-never";

/** A kind's full name and its two-letter badge. */
export type KindLabel = Readonly<{ name: string; badge: string }>;

/** The label for a standard view kind, or for a view that specializes none. */
export function kindLabel(kind: ViewKind | null): KindLabel {
  if (kind === null) {
    return { name: "View", badge: "V" };
  }
  switch (kind) {
    case "action-flow":
      return { name: "Action Flow View", badge: "AF" };
    case "browser":
      return { name: "Browser View", badge: "BR" };
    case "general":
      return { name: "General View", badge: "GV" };
    case "geometry":
      return { name: "Geometry View", badge: "GE" };
    case "grid":
      return { name: "Grid View", badge: "GR" };
    case "interconnection":
      return { name: "Interconnection View", badge: "IV" };
    case "sequence":
      return { name: "Sequence View", badge: "SQ" };
    case "state-transition":
      return { name: "State Transition View", badge: "ST" };
    default:
      return assertNever(kind);
  }
}

/** What an element reference is called on screen: its name, or what the author wrote. */
export function refName(ref: ElementRef): string {
  return ref.kind === "resolved" ? (ref.name ?? "(unnamed)") : ref.written;
}

/** The view's caption: the first thing it exposes, or else its own name. */
export function viewCaption(view: ViewSummary): string {
  const [first] = view.exposes;
  if (first !== undefined) {
    return refName(first.target);
  }
  return view.name ?? "(unnamed view)";
}
