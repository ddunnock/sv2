// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * How an element is addressed across the boundary.
 *
 * ADR-0016 settles what an identity is: a petname such as `maple-sunrise-314`,
 * carried in an inline note in the declaring file. §4.4's `ElementId` is exactly
 * that — the first row of ADR-0016's "Identity scope" table, and the only row
 * that is stored.
 *
 * THE OTHER THREE ROWS ARE NOT PETNAMES, which is why `ElementHandle` exists
 * beside it. An owning membership is derived as `<owned-ID>/m`; an implied or
 * derived relationship is a stable hash in a separate namespace; a standard
 * library element carries a normative name-based UUID defined by KerML and is
 * never assigned a petname. A diagram edge needs a handle for a derived
 * relationship and an inherited feature needs one for a library element, so a
 * type that admitted only petnames would be wrong for both on the first real
 * model. `ElementId` remains the brand §4.4 names; `ElementHandle` is the thing
 * a component actually receives.
 *
 * THE FIFTH ARM IS TRANSIENT. ADR-0020 decided that identity is allocated when
 * a workspace opens, so `unidentified` describes the window between parsing a
 * file and writing its notes, and nothing else. It is not a steady state: no
 * element reachable from a loaded workspace carries it, which is that ADR's
 * FIT-3. Components are written for the four identified arms; anything that
 * renders during loading renders a loading state, not an element.
 *
 * MINTING SITES (§4.4): `mintElementId` and `mintViewId` in this module, and
 * nothing else. Both are reached only from the schema beside them, after the
 * pattern has matched. `.brand()` is not used for the reason §4.3 rule 1 gives.
 */

import { z } from "zod";

import { type TextSpan, TextSpanSchema } from "./offset";

/**
 * The ADR-0016 petname grammar: two lowercase words and three digits.
 *
 * Anchored at both ends, so a qualified name that merely contains something
 * petname-shaped is not one. That is the substitution §4.4 says this prevents.
 */
const PETNAME = /^[a-z]+-[a-z]+-[0-9]{3}$/;

/** A stored element identity, as ADR-0016 defines it. */
export type ElementId = string & z.core.$brand<"ElementId">;

/** A view identity. Also a petname: a view is a declared element like any other. */
export type ViewId = string & z.core.$brand<"ViewId">;

/**
 * The one function that mints an `ElementId` (§4.4).
 *
 * Reached only from `ElementIdSchema`, after `PETNAME` has matched, so the
 * value is already what the brand claims.
 */
function mintElementId(matched: string): ElementId {
  // biome-ignore lint/nursery/noUnsafeTypeAssertion: the one minting site for ElementId (§4.4); the schema matches PETNAME before reaching here.
  return matched as ElementId;
}

/** The one function that mints a `ViewId` (§4.4). See `mintElementId`. */
function mintViewId(matched: string): ViewId {
  // biome-ignore lint/nursery/noUnsafeTypeAssertion: the one minting site for ViewId (§4.4); the schema matches PETNAME before reaching here.
  return matched as ViewId;
}

/** A stored element identity. */
export const ElementIdSchema: z.ZodType<ElementId, string> = z
  .string()
  .regex(PETNAME)
  .transform(mintElementId);

/**
 * A view identity.
 *
 * Separately branded from `ElementId` although the grammar is the same, because
 * ADR-0017 keys a layout sidecar by view and a component that passed an element
 * identity there would silently address the wrong file.
 */
export const ViewIdSchema: z.ZodType<ViewId, string> = z
  .string()
  .regex(PETNAME)
  .transform(mintViewId);

/** However an element is addressed. The four identified arms, plus loading. */
export type ElementHandle =
  | Readonly<{ kind: "petname"; id: ElementId }>
  | Readonly<{ kind: "membership"; of: ElementId }>
  | Readonly<{ kind: "derived"; hash: string }>
  | Readonly<{ kind: "library"; uuid: string }>
  | Readonly<{ kind: "unidentified"; span: TextSpan }>;

// The members stay module-local and unexported. That is what lets
// `z.discriminatedUnion` keep its inferred member types under
// isolatedDeclarations — see §4.3 rule 1.

/** A user element or explicit relationship: ADR-0016 row 1, the stored case. */
const petnameArm = z.strictObject({
  kind: z.literal("petname"),
  id: ElementIdSchema,
});

/**
 * The owning membership of an identified element: ADR-0016 row 2.
 *
 * Carries the owned element's identity rather than the derived `<owned-ID>/m`
 * string, because the derivation is a rule and not a second identifier. A
 * consumer that wants the string can build it; one that wants the owned element
 * has it without parsing a suffix back off.
 */
const membershipArm = z.strictObject({
  kind: z.literal("membership"),
  of: ElementIdSchema,
});

/**
 * An implied or derived relationship: ADR-0016 row 3.
 *
 * The ADR fixes what is hashed — source, relationship kind, target, ordinal —
 * and that the result lives in a namespace separate from petnames. It does not
 * fix the encoding, so this checks the part that is decided: a derived handle
 * must not be mistakable for a petname, which is what "separate namespace"
 * means and the only half of it that is checkable today. When `sv2-resolve`
 * fixes the encoding, this tightens to it.
 */
const derivedArm = z.strictObject({
  kind: z.literal("derived"),
  hash: z
    .string()
    .min(1)
    .refine((h) => !PETNAME.test(h), {
      message: "a derived identity must not be shaped like a petname (ADR-0016)",
    }),
});

/** A standard library element: ADR-0016 row 4, a normative KerML name-based UUID. */
const libraryArm = z.strictObject({
  kind: z.literal("library"),
  uuid: z.uuid(),
});

/**
 * An element parsed but not yet allocated an identity (ADR-0020).
 *
 * The span locates it in the file it was read from and is meaningful only
 * there, which is the other reason this arm cannot outlive loading: it is not
 * addressable across a workspace. Nothing persists a handle in this shape.
 */
const unidentifiedArm = z.strictObject({
  kind: z.literal("unidentified"),
  span: TextSpanSchema,
});

/** However an element is addressed. */
export const ElementHandleSchema: z.ZodType<ElementHandle, unknown> = z.discriminatedUnion("kind", [
  petnameArm,
  membershipArm,
  derivedArm,
  libraryArm,
  unidentifiedArm,
]);

/**
 * A handle that can be persisted: every arm except `unidentified`.
 *
 * ADR-0017 keys sidecar records by ADR-0016 identities, which is all four
 * identified rows — an edge can be an implied relationship, and a view can
 * expose a library element. What it can never key by is a span: that is
 * meaningful only in the file it came from, and ADR-0020 makes it transient.
 * The type excludes the arm, so a persisted record cannot be built from one.
 */
export type DurableHandle = Exclude<ElementHandle, { kind: "unidentified" }>;

/** A handle that can be persisted. */
export const DurableHandleSchema: z.ZodType<DurableHandle, unknown> = z.discriminatedUnion("kind", [
  petnameArm,
  membershipArm,
  derivedArm,
  libraryArm,
]);
