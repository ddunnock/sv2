// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * A view, as the Views list and the view tabs read it (UI-03, UI-04).
 *
 * RUST COUNTERPART: none yet. The owner is `sv2-resolve`: a view's kind comes
 * from what its definition specializes, and what it exposes is a reference that
 * may not resolve, and both are resolver work.
 *
 * MEMBERSHIP IS IN THE MODEL (ADR-0006). What a view shows is its `expose`
 * relationships, which are model content; nothing here comes from a sidecar.
 * The mockup names each view by its kind and what it exposes — "GV
 * ThermalControl", "IV thermalSubsystem" — which is why `exposes` is on the
 * summary rather than waiting for a detail query.
 *
 * THE KIND UNION IS COMPLETE, INCLUDING WHAT IS NOT BUILT. OD-03 leaves the
 * State Transition view undesigned and ADR-0008 builds structure first, but the
 * union carries all eight standard views anyway. A deliberately incomplete
 * union gets widened under pressure, one member at a time, each widening a wire
 * change; a complete one lets `ViewArea` dispatch exhaustively on day one and
 * render `<Unavailable>` for every kind not yet drawn.
 *
 * NOT HERE YET: `render` (a `ViewRenderingMembership`) and viewpoints. ADR-0006
 * lists both, and neither changes which view the dispatcher picks.
 */

import { z } from "zod";

import { type ElementRef, ElementRefSchema } from "./element";
import { type ViewId, ViewIdSchema } from "./element-id";

/**
 * The eight standard views of SysML v2, clause 9.2.20.2 — the
 * `StandardViewDefinitions` package, whose subclauses .1 to .8 are exactly
 * these, with no gaps. Each wire value is the definition's name in kebab case:
 *
 * | Wire value         | Library definition    | Clause     |
 * |--------------------|-----------------------|------------|
 * | `action-flow`      | `ActionFlowView`      | 9.2.20.2.1 |
 * | `browser`          | `BrowserView`         | 9.2.20.2.2 |
 * | `general`          | `GeneralView`         | 9.2.20.2.3 |
 * | `geometry`         | `GeometryView`        | 9.2.20.2.4 |
 * | `grid`             | `GridView`            | 9.2.20.2.5 |
 * | `interconnection`  | `InterconnectionView` | 9.2.20.2.6 |
 * | `sequence`         | `SequenceView`        | 9.2.20.2.7 |
 * | `state-transition` | `StateTransitionView` | 9.2.20.2.8 |
 *
 * Closed, because the standard library closes it: a ninth standard view is a
 * new specification, not a newer core.
 */
export const VIEW_KINDS = [
  "action-flow",
  "browser",
  "general",
  "geometry",
  "grid",
  "interconnection",
  "sequence",
  "state-transition",
] as const;

/** One of the eight standard views. */
export type ViewKind = (typeof VIEW_KINDS)[number];

/** One of the eight standard views. */
export const ViewKindSchema: z.ZodType<ViewKind, unknown> = z.enum(VIEW_KINDS);

/**
 * One `expose` of a view.
 *
 * `kind` is the metaclass — `MembershipExpose` exposes one element,
 * `NamespaceExpose` the members of a namespace — and `isRecursive` is
 * `Import::isRecursive`, the `::**` form. Visibility and `isImportAll` are not
 * carried: the pinned metamodel fixes them for every `Expose` (always
 * `protected`, always import-all), and a constant on the wire is a field that
 * can only be wrong.
 *
 * `target` may be unresolved, because ADR-0002 admits a view whose expose
 * names something that does not exist yet. The view still lists; its tab says
 * what it failed to find.
 */
export type ViewExpose = Readonly<{
  kind: "membership" | "namespace";
  target: ElementRef;
  isRecursive: boolean;
}>;

/** One `expose` of a view. */
export const ViewExposeSchema: z.ZodType<ViewExpose, unknown> = z.strictObject({
  kind: z.enum(["membership", "namespace"]),
  target: ElementRefSchema,
  isRecursive: z.boolean(),
});

/**
 * Enough to list a view, open its tab, and pick its renderer.
 *
 * `kind` is null when the view's definition specializes none of the eight — a
 * plain `view def` is legal SysML, and a view the tool has no renderer for is
 * still a view in the model. The dispatcher renders it as unavailable; it does
 * not vanish from the list. Which kind a definition that specializes two of
 * them gets is the resolver's rule, not something re-derived here.
 *
 * `name` is the view usage's `Element::name`, `[0..1]` like any element's.
 */
export type ViewSummary = Readonly<{
  id: ViewId;
  name: string | null;
  kind: ViewKind | null;
  exposes: readonly ViewExpose[];
}>;

/** Enough to list a view, open its tab, and pick its renderer. */
export const ViewSummarySchema: z.ZodType<ViewSummary, unknown> = z.strictObject({
  id: ViewIdSchema,
  name: z.string().nullable(),
  kind: ViewKindSchema.nullable(),
  exposes: z.array(ViewExposeSchema).readonly(),
});
