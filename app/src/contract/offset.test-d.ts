// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Type tests for the offset brand (§11 rule 11).
 *
 * These are never run. `tsc` checks them, and a `@ts-expect-error` whose error
 * has stopped happening is itself an error — which is the point: a brand that
 * silently widened to `number` would pass every value test in
 * `offset.test.ts` while proving nothing, because every assertion in that file
 * would still hold.
 *
 * Values are introduced with `declare const` rather than an assertion, so that
 * proving the brand works does not require the one thing the brand exists to
 * forbid.
 */

import type { TextSpan, Utf16Offset } from "./offset";

declare const offset: Utf16Offset;
declare const plain: number;
declare const span: TextSpan;

/** A plain number is not a validated position, which is the whole brand. */
// @ts-expect-error a number that has not been through Utf16OffsetSchema is not an offset
export const notAssignableFromNumber: Utf16Offset = plain;

/** A position is still a number, so comparison and arithmetic keep working. */
export const assignableToNumber: number = offset;

/** The span is readonly, so nothing downstream can move a diagnostic. */
// @ts-expect-error TextSpan is Readonly; §4.6 rule 1 moves readonly values inward
span.start = offset;

/** A bare pair of numbers is not a span, for the same reason as above. */
// @ts-expect-error both ends must be validated positions, not plain numbers
export const notAssignableFromPlainPair: TextSpan = { start: 0, end: 1 };
