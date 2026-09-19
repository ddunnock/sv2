// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * An element, as the Specification sidebar and the Elements tree read it.
 *
 * RUST COUNTERPART: none yet. The owner is `sv2-resolve`, the layer that
 * produces resolved references and derived properties; `sv2-hir` and
 * `sv2-resolve` are empty stubs today. Every field below names the metamodel
 * property it mirrors in the pinned metamodel (`vendor/omg/20250201`), so when
 * the Rust type is written it has something to be compared against other than
 * this file.
 *
 * AN UNRESOLVED REFERENCE IS NOT A RESOLVED ONE WITH A FLAG. ADR-0002 admits an
 * element whose references do not resolve and requires that "an unresolved
 * reference should not be representable as a resolved one". `ElementRef` is
 * that requirement as a union: a resolved arm that has a target and an
 * unresolved arm that has only what the author wrote and the code saying why.
 * A consumer cannot reach a target without first narrowing.
 *
 * EVERY FEATURE ROW CARRIES ITS ORIGIN, even though the UI ships owned-only.
 * OD-04 asks whether inherited features appear; the mockup's own SCR-02 already
 * lists `controller`'s ports, which are features of `ThermalController`. That
 * is a resolver question with a wire consequence, and one field now is cheaper
 * than a wire change and a fixture regeneration later.
 *
 * PRESENTATION IS NOT ON THE WIRE. The mockup's "attr", "Satisfies" and
 * "Satisfied by" are renderings of a metaclass and a direction. They are
 * derived in `model/`, so relabelling a row never changes the contract.
 *
 * NOT HERE YET: a feature's value (`= 0.5` in SCR-02), feature chains as a
 * reference target (`heater.pwrIn`), and per-element diagnostic counts for the
 * tree. Each arrives with the query that needs it.
 */

import { z } from "zod";

import { type Diagnostic, DiagnosticCodeSchema, DiagnosticSchema } from "./diagnostic";
import { type ElementHandle, ElementHandleSchema } from "./element-id";
import { type TextSpan, TextSpanSchema } from "./offset";

/**
 * The name of a metaclass, such as `PartDefinition`.
 *
 * The pinned metamodel defines the set — `vendor/omg/20250201/SysML.json`
 * holds one `$defs` entry per metaclass.
 */
export type Metaclass = string;

/**
 * A metaclass name, checked by shape only.
 *
 * Every name in the pinned metamodel is PascalCase letters (all 183 `$defs`
 * in vendor/omg/20250201/SysML.json match), so this rejects a typo'd case
 * or a qualified name. It does not check membership: a closed list
 * typed out by hand here would be unsourced. It closes when the Rust type
 * exists and check_ipc_contract.py compares the two enums (§4.2).
 */
const METACLASS = /^[A-Z][A-Za-z]*$/;
export const MetaclassSchema: z.ZodType<Metaclass, unknown> = z.string().regex(METACLASS);

/**
 * `VisibilityKind`, verbatim from the pinned metamodel.
 *
 * Closed, because the metamodel closes it: `SysML.json` gives exactly these
 * three as an `enum`, and a fourth would be a metamodel change, not a newer
 * core.
 */
export const VISIBILITIES = ["public", "protected", "private"] as const;

/** `VisibilityKind`. */
export type Visibility = (typeof VISIBILITIES)[number];

/** `VisibilityKind`. */
export const VisibilitySchema: z.ZodType<Visibility, unknown> = z.enum(VISIBILITIES);

/**
 * A reference from one element to another, resolved or not (ADR-0002).
 *
 * `name` on the resolved arm is the target's `Element::name` — its effective
 * name, which is derived, and which is why `~TempPort` needs no conjugation
 * flag here. The pinned metamodel defines `ConjugatedPortDefinition::
 * effectiveName` as the original's name with `~` prepended: the target is a
 * different element whose name already says so. It is nullable because
 * `Element::name` is `[0..1]`; an unnamed target is legal.
 *
 * The unresolved arm is the shape §4.3 gives as its own example. `written` is
 * model text, which §9.3 keeps out of logs.
 */
export type ElementRef =
  | Readonly<{ kind: "resolved"; target: ElementHandle; name: string | null }>
  | Readonly<{ kind: "unresolved"; written: string; code: string }>;

const resolvedArm = z.strictObject({
  kind: z.literal("resolved"),
  target: ElementHandleSchema,
  name: z.string().nullable(),
});

const unresolvedArm = z.strictObject({
  kind: z.literal("unresolved"),
  written: z.string(),
  code: DiagnosticCodeSchema,
});

/** A reference from one element to another, resolved or not. */
export const ElementRefSchema: z.ZodType<ElementRef, unknown> = z.discriminatedUnion("kind", [
  resolvedArm,
  unresolvedArm,
]);

/**
 * One end of a multiplicity range, as written.
 *
 * Carried as written rather than evaluated: a bound is an expression in the
 * abstract syntax, and `[n]` is legal. `literal` and `unbounded` are the two
 * cases the diagram needs without an evaluator — a stacked box for an upper
 * bound above one — and `expression` carries the rest as text until one exists.
 */
export type MultiplicityBound =
  | Readonly<{ kind: "literal"; value: number }>
  | Readonly<{ kind: "unbounded" }>
  | Readonly<{ kind: "expression"; text: string }>;

const literalBound = z.strictObject({
  kind: z.literal("literal"),
  // §4.2: u32 or smaller on the wire.
  value: z.number().int().nonnegative().max(0xffff_ffff),
});
const unboundedBound = z.strictObject({ kind: z.literal("unbounded") });
const expressionBound = z.strictObject({ kind: z.literal("expression"), text: z.string() });

/** One end of a multiplicity range, as written. */
export const MultiplicityBoundSchema: z.ZodType<MultiplicityBound, unknown> = z.discriminatedUnion(
  "kind",
  [literalBound, unboundedBound, expressionBound],
);

/**
 * A multiplicity, as written: `[1]`, `[0..*]`, `[2..n]`.
 *
 * `lower` is null when only one bound was written. What `[3]` means for the
 * lower bound is the core's to decide, and deciding it here would be a second
 * opinion in a second language — the same reason diagnostic severity is
 * carried rather than re-derived.
 */
export type Multiplicity = Readonly<{
  lower: MultiplicityBound | null;
  upper: MultiplicityBound;
}>;

/** A multiplicity, as written. */
export const MultiplicitySchema: z.ZodType<Multiplicity, unknown> = z.strictObject({
  lower: MultiplicityBoundSchema.nullable(),
  upper: MultiplicityBoundSchema,
});

/**
 * Where a feature row came from (OD-04).
 *
 * `inherited` names the type it was inherited from, so a row can say "from
 * `ThermalController`" and a click can go there.
 */
export type FeatureOrigin =
  | Readonly<{ kind: "owned" }>
  | Readonly<{ kind: "inherited"; from: ElementHandle }>;

const ownedOrigin = z.strictObject({ kind: z.literal("owned") });
const inheritedOrigin = z.strictObject({
  kind: z.literal("inherited"),
  from: ElementHandleSchema,
});

/** Where a feature row came from. */
export const FeatureOriginSchema: z.ZodType<FeatureOrigin, unknown> = z.discriminatedUnion("kind", [
  ownedOrigin,
  inheritedOrigin,
]);

/**
 * One row of the Owned features table (IX-05): kind, name, type, multiplicity.
 *
 * `types` is a list because `Feature::type` is `[0..*]`. The mockup draws one,
 * and an empty list is an untyped feature — which is legal, and not the same
 * as an unresolved type. The wire does not narrow what the metamodel allows.
 */
export type FeatureRow = Readonly<{
  handle: ElementHandle;
  metaclass: Metaclass;
  name: string | null;
  types: readonly ElementRef[];
  multiplicity: Multiplicity | null;
  origin: FeatureOrigin;
}>;

/** One row of the Owned features table. */
export const FeatureRowSchema: z.ZodType<FeatureRow, unknown> = z.strictObject({
  handle: ElementHandleSchema,
  metaclass: MetaclassSchema,
  name: z.string().nullable(),
  types: z.array(ElementRefSchema).readonly(),
  multiplicity: MultiplicitySchema.nullable(),
  origin: FeatureOriginSchema,
});

/**
 * One row of the Relationships section (IX-05).
 *
 * `handle` is the relationship element itself — often a derived one, ADR-0016
 * row 3 — and `other` is the element at the far end, which may not resolve.
 * `direction` says which end this element is at, and is what turns one
 * `SatisfyRequirementUsage` into "Satisfies" on one sidebar and "Satisfied by"
 * on the other. `isImplied` is `Relationship::isImplied`: the mockup's
 * "(implicit)".
 */
export type RelationshipRow = Readonly<{
  handle: ElementHandle;
  metaclass: Metaclass;
  direction: "outgoing" | "incoming";
  other: ElementRef;
  isImplied: boolean;
}>;

/** One row of the Relationships section. */
export const RelationshipRowSchema: z.ZodType<RelationshipRow, unknown> = z.strictObject({
  handle: ElementHandleSchema,
  metaclass: MetaclassSchema,
  direction: z.enum(["outgoing", "incoming"]),
  other: ElementRefSchema,
  isImplied: z.boolean(),
});

/**
 * Enough to name an element in a tree, a tab or a sidebar header.
 *
 * `name`, `shortName` and `qualifiedName` are the `Element` properties of the
 * same names, each `[0..1]`. The mockup's "Req ID TC-001" is the short name,
 * declared as `<'TC-001'>`.
 */
export type ElementSummary = Readonly<{
  handle: ElementHandle;
  metaclass: Metaclass;
  name: string | null;
  shortName: string | null;
  qualifiedName: string | null;
}>;

/** Enough to name an element. */
export const ElementSummarySchema: z.ZodType<ElementSummary, unknown> = z.strictObject({
  handle: ElementHandleSchema,
  metaclass: MetaclassSchema,
  name: z.string().nullable(),
  shortName: z.string().nullable(),
  qualifiedName: z.string().nullable(),
});

/**
 * What an element is, in the three shapes the General section draws.
 *
 * A union rather than two nullable blocks, because `Feature` specializes
 * `Type`: a feature block without a type block would be representable and
 * meaningless. `isAbstract` and `specializes` belong to every `Type`;
 * `isComposite` to every `Feature`. `isReference` is absent on purpose — the
 * pinned metamodel derives `Usage::isReference` as `isComposite = false`, and
 * a derived value on the wire beside its source is two values that can
 * disagree.
 */
export type ElementFacet =
  | Readonly<{ kind: "element" }>
  | Readonly<{
      kind: "type";
      isAbstract: boolean;
      specializes: readonly RelationshipRow[];
    }>
  | Readonly<{
      kind: "feature";
      isAbstract: boolean;
      specializes: readonly RelationshipRow[];
      types: readonly ElementRef[];
      multiplicity: Multiplicity | null;
      isComposite: boolean;
      redefines: readonly ElementRef[];
    }>;

const elementFacet = z.strictObject({ kind: z.literal("element") });
const typeFacet = z.strictObject({
  kind: z.literal("type"),
  isAbstract: z.boolean(),
  specializes: z.array(RelationshipRowSchema).readonly(),
});
const featureFacet = z.strictObject({
  kind: z.literal("feature"),
  isAbstract: z.boolean(),
  specializes: z.array(RelationshipRowSchema).readonly(),
  types: z.array(ElementRefSchema).readonly(),
  multiplicity: MultiplicitySchema.nullable(),
  isComposite: z.boolean(),
  redefines: z.array(ElementRefSchema).readonly(),
});

/** What an element is. */
export const ElementFacetSchema: z.ZodType<ElementFacet, unknown> = z.discriminatedUnion("kind", [
  elementFacet,
  typeFacet,
  featureFacet,
]);

/**
 * Where an element's text is.
 *
 * `file` is workspace-relative. The span locates the declaration in it, which
 * is what IX-08 scrolls to and what the Element Source view is a window onto.
 */
export type SourceLocation = Readonly<{ file: string; span: TextSpan }>;

/** Where an element's text is. */
export const SourceLocationSchema: z.ZodType<SourceLocation, unknown> = z.strictObject({
  file: z.string().min(1),
  span: TextSpanSchema,
});

/**
 * Everything the Specification tab shows for one element (IX-05).
 *
 * `owner` and `visibility` are null for a root namespace, which has neither;
 * visibility belongs to the owning membership, not to the element. `location`
 * is null for an element with no text in the workspace — a library element,
 * or an implied one. `documentation` is a list because an element may own more
 * than one `Documentation`, and its bodies are model text (§9.3).
 *
 * `diagnostics` are this element's, carried whole. The sidebar banner, the
 * diagram underline and the Problems row are three renderings of one list
 * (IX-10), and ADR-0002 is why they decorate rather than hide.
 */
export type ElementDetail = Readonly<{
  summary: ElementSummary;
  facet: ElementFacet;
  owner: ElementRef | null;
  visibility: Visibility | null;
  location: SourceLocation | null;
  documentation: readonly string[];
  ownedFeatures: readonly FeatureRow[];
  relationships: readonly RelationshipRow[];
  diagnostics: readonly Diagnostic[];
}>;

/** Everything the Specification tab shows for one element. */
export const ElementDetailSchema: z.ZodType<ElementDetail, unknown> = z.strictObject({
  summary: ElementSummarySchema,
  facet: ElementFacetSchema,
  owner: ElementRefSchema.nullable(),
  visibility: VisibilitySchema.nullable(),
  location: SourceLocationSchema.nullable(),
  documentation: z.array(z.string()).readonly(),
  ownedFeatures: z.array(FeatureRowSchema).readonly(),
  relationships: z.array(RelationshipRowSchema).readonly(),
  diagnostics: z.array(DiagnosticSchema).readonly(),
});
