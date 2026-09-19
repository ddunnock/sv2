// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Type tests for the view contract (§11 rule 11).
 *
 * Never run; `tsc` checks them. The claim that matters is the one Phase 4's
 * dispatcher rests on: a switch over `ViewKind` that forgets a kind does not
 * compile. That is a property of the type, not of any value.
 */

import type { ElementId, ViewId } from "./element-id";
import type { ViewKind, ViewSummary } from "./view";

declare const kind: ViewKind;
declare const elementId: ElementId;
declare const summary: ViewSummary;

/** Every kind handled: this compiles, and is the shape `ViewArea` will take. */
function handled(k: ViewKind): string {
  switch (k) {
    case "action-flow":
    case "browser":
    case "general":
    case "geometry":
    case "grid":
    case "interconnection":
    case "sequence":
    case "state-transition":
      return k;
    default: {
      const unreachable: never = k;
      return unreachable;
    }
  }
}
export const handledKind: string = handled(kind);

/** One kind forgotten: the leftover is not `never`, so this does not compile. */
function forgetsStateTransition(k: ViewKind): string {
  switch (k) {
    case "action-flow":
    case "browser":
    case "general":
    case "geometry":
    case "grid":
    case "interconnection":
    case "sequence":
      return k;
    default: {
      // @ts-expect-error "state-transition" is left over, and it is not never
      const unreachable: never = k;
      return unreachable;
    }
  }
}
export const forgottenKind: string = forgetsStateTransition(kind);

/** A view is addressed by its view identity, not an element identity (ADR-0017). */
// @ts-expect-error ElementId is not a ViewId, although both are petnames
export const notAnElementId: ViewSummary = { ...summary, id: elementId };

/** A kind is null only when there is no standard kind, and must be checked for. */
// @ts-expect-error kind may be null; the dispatcher has to handle that first
export const kindNotChecked: ViewKind = summary.kind;

/** The summary's id is the view brand. */
export const idIsAViewId: ViewId = summary.id;
