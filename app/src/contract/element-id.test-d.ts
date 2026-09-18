// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Type tests for the identity brands and the handle union (§11 rule 11).
 *
 * Never run; `tsc` checks them. The substitutions proved here are the ones
 * §4.4's table says the brands exist to prevent, and every one of them would
 * pass a value test: a qualified name, a view identity and an element identity
 * are all just strings at run time.
 */

import type { ElementHandle, ElementId, ViewId } from "./element-id";

declare const elementId: ElementId;
declare const viewId: ViewId;
declare const plain: string;
declare const handle: ElementHandle;

/** §4.4: prevents "a qualified name, or any string, used as an identity". */
// @ts-expect-error a string that has not been through ElementIdSchema is not an identity
export const notFromString: ElementId = plain;

/** §4.4: prevents "a view identifier used as an element identifier". */
// @ts-expect-error ViewId and ElementId share a grammar and are not interchangeable
export const notFromViewId: ElementId = viewId;

/** And the substitution in the other direction, which ADR-0017 would hide. */
// @ts-expect-error an element identity is not a view identity
export const notFromElementId: ViewId = elementId;

/** Both are still strings, so they can be compared, keyed and rendered. */
export const stillAString: string = elementId;

/**
 * A handle is not an identity. Reaching for `.id` without discriminating is the
 * mistake ADR-0016's identity-scope table makes easy, and this is what stops it.
 */
// @ts-expect-error only the petname arm has an id; the union must be narrowed first
export const noUncheckedId: ElementId = handle.id;

/** Narrowing works, and gives the identity. */
export const narrowed: ElementId | undefined = handle.kind === "petname" ? handle.id : undefined;

/** The arms are closed: a kind ADR-0016 does not define is not a handle. */
// @ts-expect-error "guid" is not one of the five arms
export const noInventedKind: ElementHandle = { kind: "guid", id: plain };
