// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * A view's layout sidecar, as it crosses to the diagram (ADR-0017).
 *
 * RUST COUNTERPART: none yet. The Rust side owns the file — it reads and writes
 * `views/<view-id>.layout.jsonl` — and sends one view's records here.
 *
 * THE WIRE IS NOT THE FILE. ADR-0017 fixes the file: JSON Lines, a header then
 * records sorted by `(kind, element)`, snake_case keys, a string per element.
 * The wire follows §4.2 instead: camelCase through serde, handles rather than
 * bare strings, and records grouped by kind. Grouping loses nothing, because
 * R-2's sort order is recoverable by sorting, and it is what lets each kind be
 * typed without a string-kinded catch-all arm defeating narrowing.
 *
 * UNKNOWN THINGS ARE CARRIED, NEVER DROPPED (R-6). A newer instance may have
 * written a field or a record kind this build does not know, and an older build
 * must not destroy it. So every known record is a `z.looseObject` — the one
 * exception §4.3 rule 2 names — and records of an unknown kind arrive in
 * `unknown`, whole. Unknown fields keep the file's spelling; they are opaque
 * here, and only Rust ever writes them back.
 *
 * NOT HERE YET: the style sidecar. ADR-0017 names `views/<view-id>.style.jsonl`
 * and says what it holds, but gives no record shape for it, and inventing one
 * here would be a claim with no source.
 *
 * MINTING SITES (§4.4): `mintGridUnit` in this module, and nothing else.
 */

import { z } from "zod";

import { type DurableHandle, DurableHandleSchema, type ViewId, ViewIdSchema } from "./element-id";

/**
 * A coordinate or dimension, in grid units (R-1).
 *
 * Branded because the diagram also handles pixels, and both are integers. A
 * pixel value written into a layout record is the substitution §4.4 names this
 * brand for: it would survive every check and put the node somewhere else.
 *
 * Signed, because nothing in ADR-0017 confines a view to one quadrant. Bounded
 * to `i32`, the §4.2 limit a JavaScript number carries exactly.
 */
export type GridUnit = number & z.core.$brand<"GridUnit">;

/**
 * The one function that mints a `GridUnit` (§4.4).
 *
 * Reached only from `GridUnitSchema`, after the integer and range checks.
 */
function mintGridUnit(checked: number): GridUnit {
  // biome-ignore lint/nursery/noUnsafeTypeAssertion: the one minting site for GridUnit (§4.4); the schema checks integer and range before reaching here.
  return checked as GridUnit;
}

const I32_MIN = -(2 ** 31);
const I32_MAX = 2 ** 31 - 1;

/** A coordinate or dimension, in grid units. */
export const GridUnitSchema: z.ZodType<GridUnit, number> = z
  .number()
  .int()
  .min(I32_MIN)
  .max(I32_MAX)
  .transform(mintGridUnit);

/** A width or height: a grid quantity that cannot be negative. */
const DimensionSchema: z.ZodType<GridUnit, number> = z
  .number()
  .int()
  .nonnegative()
  .max(I32_MAX)
  .transform(mintGridUnit);

/**
 * Where a record came from (R-4): laid out by the engine, or placed by a person.
 *
 * Re-layout discards `auto` and keeps `pinned`, so this is the difference
 * between a position that may move and one a person chose.
 */
export const RECORD_SOURCES = ["auto", "pinned"] as const;

/** Where a record came from. */
export type RecordSource = (typeof RECORD_SOURCES)[number];

const RecordSourceSchema: z.ZodType<RecordSource, unknown> = z.enum(RECORD_SOURCES);

/**
 * The schema version this build reads (R-7).
 *
 * A literal, not a minimum. R-7 increments `schema` only on a breaking change,
 * so a file at 2 is one whose records this build would misread. Failing the
 * parse is the honest answer, and ADR-0017 DD-4 makes it a safe one: a layout
 * that cannot be read degrades to auto-layout, with no model impact.
 */
export const LAYOUT_SCHEMA = 1;

/** Unknown fields, carried through (R-6). */
type Carried = Readonly<Record<string, unknown>>;

/**
 * True when a known record does not carry the file's `kind` key.
 *
 * On the wire the list a record is in is its kind. A `kind` that crossed anyway
 * would be carried as an unknown field, so a node record misfiled among the
 * edges would pass as an edge with an opaque `kind: "node"`. Rejecting the key
 * makes the misfiling loud.
 */
function hasNoKind(record: Readonly<Record<string, unknown>>): boolean {
  return !Object.hasOwn(record, "kind");
}

const KIND_ON_KNOWN = { message: "a known record's kind is its list; it carries no kind key" };

/**
 * The header line: which view, which engine, and the grid.
 *
 * `grid` is the size of one grid unit, and is the one integer here that is not
 * in grid units — which is exactly why it is a plain number. `engine` and
 * `engineVersion` are R-5's stamp, so a snapshot that predates an engine change
 * surfaces a prompt rather than a silent reflow.
 */
export type LayoutHeader = Readonly<{
  schema: typeof LAYOUT_SCHEMA;
  view: ViewId;
  engine: string;
  engineVersion: string;
  grid: number;
}> &
  Carried;

const LayoutHeaderSchema: z.ZodType<LayoutHeader, unknown> = z
  .looseObject({
    schema: z.literal(LAYOUT_SCHEMA),
    view: ViewIdSchema,
    engine: z.string().min(1),
    engineVersion: z.string().min(1),
    grid: z.number().int().positive().max(0xffff_ffff),
  })
  .refine(hasNoKind, KIND_ON_KNOWN);

/** A node's box. */
export type NodeRecord = Readonly<{
  element: DurableHandle;
  x: GridUnit;
  y: GridUnit;
  w: GridUnit;
  h: GridUnit;
  src: RecordSource;
}> &
  Carried;

const NodeRecordSchema: z.ZodType<NodeRecord, unknown> = z
  .looseObject({
    element: DurableHandleSchema,
    x: GridUnitSchema,
    y: GridUnitSchema,
    w: DimensionSchema,
    h: DimensionSchema,
    src: RecordSourceSchema,
  })
  .refine(hasNoKind, KIND_ON_KNOWN);

/** A point, as ADR-0017's `[x, y]` pair. */
export type GridPoint = readonly [GridUnit, GridUnit];

/** An edge's route. An empty list is a straight line between its ends. */
export type EdgeRecord = Readonly<{
  element: DurableHandle;
  waypoints: readonly GridPoint[];
  src: RecordSource;
}> &
  Carried;

const EdgeRecordSchema: z.ZodType<EdgeRecord, unknown> = z
  .looseObject({
    element: DurableHandleSchema,
    waypoints: z.array(z.tuple([GridUnitSchema, GridUnitSchema]).readonly()).readonly(),
    src: RecordSourceSchema,
  })
  .refine(hasNoKind, KIND_ON_KNOWN);

/**
 * A node's presentation state: collapsed, and which compartments show.
 *
 * Compartment names are open. ADR-0017's example has `parts` and `ports`, and
 * which compartments exist is a property of the notation, not of this file.
 */
export type StateRecord = Readonly<{
  element: DurableHandle;
  collapsed: boolean;
  compartments: readonly string[];
}> &
  Carried;

const StateRecordSchema: z.ZodType<StateRecord, unknown> = z
  .looseObject({
    element: DurableHandleSchema,
    collapsed: z.boolean(),
    compartments: z.array(z.string().min(1)).readonly(),
  })
  .refine(hasNoKind, KIND_ON_KNOWN);

/** The record kinds this build knows, which `unknown` must not contain. */
const KNOWN_KINDS: ReadonlySet<string> = new Set(["header", "node", "edge", "state"]);

/**
 * A record of a kind this build does not know, carried whole (R-6).
 *
 * It keeps its `kind` — that is the only field it is guaranteed to have — and
 * everything else is opaque. A known kind here would be a record this build
 * could read and is not reading, so it is rejected.
 */
export type UnknownRecord = Readonly<{ kind: string }> & Carried;

const UnknownRecordSchema: z.ZodType<UnknownRecord, unknown> = z.looseObject({
  kind: z.string().refine((kind) => !KNOWN_KINDS.has(kind), {
    message: "a record of a known kind belongs in its own list",
  }),
});

/** One view's layout: the header, the records it knows, and the ones it does not. */
export type ViewLayout = Readonly<{
  header: LayoutHeader;
  nodes: readonly NodeRecord[];
  edges: readonly EdgeRecord[];
  states: readonly StateRecord[];
  unknown: readonly UnknownRecord[];
}>;

/**
 * One view's layout.
 *
 * `strictObject` at this level, `looseObject` below it. R-6 is about the
 * sidecar's records; this envelope is the IPC shape, which is ours, and an
 * unknown key on it is a typo like any other.
 */
export const ViewLayoutSchema: z.ZodType<ViewLayout, unknown> = z.strictObject({
  header: LayoutHeaderSchema,
  nodes: z.array(NodeRecordSchema).readonly(),
  edges: z.array(EdgeRecordSchema).readonly(),
  states: z.array(StateRecordSchema).readonly(),
  unknown: z.array(UnknownRecordSchema).readonly(),
});
