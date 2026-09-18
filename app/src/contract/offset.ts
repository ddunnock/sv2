// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Positions in a document, as they cross the boundary.
 *
 * ONE UNIT, AND IT IS UTF-16. The Rust core counts bytes: `sv2-syntax` uses
 * `TextRange` over `TextSize`, and its `offset` module calls itself the single
 * designated offset boundary. ADR-0013 RISK-013-2 is the off-by-N that follows
 * from having two units, and the mitigation it names is to convert at exactly
 * one place. That place is the Rust side, so **the webview never receives a
 * UTF-8 offset** and nothing here converts one.
 *
 * MINTING SITES for `Utf16Offset` (§4.4): the `.transform` in
 * `Utf16OffsetSchema` below, and nothing else. A number that reached the
 * webview any other way — an arithmetic result, a CodeMirror position, a
 * length — is a `number` until it is parsed here, and the compiler says so.
 *
 * RUST COUNTERPART: `sv2_syntax::TextRange`, re-exported from `text_size`.
 *
 * ON THE SHAPE OF THESE DECLARATIONS: the type is declared and the schema is
 * annotated against it, rather than the type being inferred from the schema.
 * That is §4.3 rule 1 as it now reads, and the reason is mechanical —
 * `isolatedDeclarations` cannot state the type of an exported Zod schema. The
 * rule carries the whole explanation; this module just follows it.
 */

import { z } from "zod";

/**
 * A position in a document, counted in UTF-16 code units from its start.
 *
 * Branded, so a plain `number` cannot be passed where a position is required.
 * That is the whole point: the two units are both integers, and nothing but the
 * type system tells them apart.
 */
export type Utf16Offset = number & z.core.$brand<"Utf16Offset">;

/**
 * The one function that mints a `Utf16Offset`, which is the shape §4.4 asks
 * for: a single named site carrying the single permitted assertion.
 *
 * Reached only from `Utf16OffsetSchema` below, after `int()` and
 * `nonnegative()` have passed, so the value is already what the brand claims.
 * At run time this returns its argument unchanged — the brand exists only in
 * the type system — which is what makes the transform lossless as §4.3
 * requires.
 */
function mintUtf16Offset(value: number): Utf16Offset {
  // biome-ignore lint/nursery/noUnsafeTypeAssertion: the one minting site for Utf16Offset (§4.4); the schema validates before reaching here, and the brand is type-only at run time.
  return value as Utf16Offset;
}

/**
 * A validated document position.
 *
 * `.transform` rather than Zod's own `.brand()`: `.brand()` returns
 * `$ZodBranded`, which is not assignable to `z.ZodType` under
 * `exactOptionalPropertyTypes`, and an exported schema must carry an explicit
 * annotation under `isolatedDeclarations`.
 */
export const Utf16OffsetSchema: z.ZodType<Utf16Offset, number> = z
  .number()
  .int()
  .nonnegative()
  .transform(mintUtf16Offset);

/**
 * A half-open range, `[start, end)`, mirroring `sv2_syntax::TextRange`.
 *
 * `start === end` is legal and meaningful: an empty span points between two
 * characters, which is where an "expected X here" diagnostic attaches, and
 * `sv2-syntax` raises those. That `start <= end` is deliberately not refined
 * here — it would run on every span on a per-keystroke path, and §4.7 checks
 * the container once rather than re-validating each element.
 */
export type TextSpan = Readonly<{
  start: Utf16Offset;
  end: Utf16Offset;
}>;

/** A half-open range of a document, in UTF-16 code units. */
export const TextSpanSchema: z.ZodType<TextSpan, { start: number; end: number }> = z.strictObject({
  start: Utf16OffsetSchema,
  end: Utf16OffsetSchema,
});
