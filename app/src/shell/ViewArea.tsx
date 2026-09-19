// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The view area (UI-04, UI-05): the open views' tabs, and the selected view.
 *
 * THE DISPATCHER COMES BEFORE ANY VIEW. `renderView` switches on every
 * `ViewKind` and ends in `assertNever`, so there is exactly one place where a
 * kind becomes a renderer. Today every case is `<Unavailable>` — no diagram
 * exists, and the plan forbids simulating one, down to an empty dotted canvas.
 * When ADR-0008's first view lands, one case in this file changes; when
 * interconnection follows, one more does.
 *
 * View tabs use manual activation: arrows move between them and Enter opens
 * one, because opening a view is not free.
 */

import type { ReactNode } from "react";

import type { ViewId } from "@/contract/element-id";
import type { ViewSummary } from "@/contract/view";
import { assertNever } from "@/model/assert-never";
import { kindLabel, viewCaption } from "@/model/view-label";

import { IslandBoundary } from "./IslandBoundary";
import { Tabs } from "./primitives/Tabs";
import { useServices } from "./services";
import { Unavailable } from "./Unavailable";

/** Props for `ViewArea`. */
export type ViewAreaProps = Readonly<{
  open: readonly ViewSummary[];
  active: ViewId | null;
  onActivate: (id: ViewId) => void;
}>;

/** The open views, as tabs over the active one. */
export function ViewArea({ open, active, onActivate }: ViewAreaProps): React.JSX.Element {
  const { report } = useServices();
  const current = open.find((view) => view.id === active) ?? open[0];
  return (
    <main aria-label="Views" className="flex min-w-0 flex-1 flex-col bg-canvas">
      {current === undefined ? (
        <p className="p-4 text-muted">No view is open. Choose one from the Views list.</p>
      ) : (
        <Tabs
          label="Open views"
          tabs={open.map((view) => ({ id: view.id, label: tabLabel(view) }))}
          selected={current.id}
          onSelect={(id) => {
            const view = open.find((candidate) => candidate.id === id);
            if (view !== undefined) {
              onActivate(view.id);
            }
          }}
          activation="manual"
          orientation="horizontal"
          panel={
            <IslandBoundary island="diagram" report={report}>
              {renderView(current)}
            </IslandBoundary>
          }
        />
      )}
    </main>
  );
}

/** "GV ThermalControl", as the mockup's tabs read. */
export function tabLabel(view: ViewSummary): string {
  return `${kindLabel(view.kind).badge} ${viewCaption(view)}`;
}

/** The one place a view kind becomes a renderer. Every kind is unavailable today. */
function renderView(view: ViewSummary): ReactNode {
  const what = `The ${kindLabel(view.kind).name} of ${viewCaption(view)}`;
  const kind = view.kind;
  if (kind === null) {
    return <Unavailable what={what} because={notYet("drawing a view of no standard kind")} />;
  }
  switch (kind) {
    case "general":
      return <Unavailable what={what} because={notYet("drawing a General View")} />;
    case "interconnection":
      return <Unavailable what={what} because={notYet("drawing an Interconnection View")} />;
    case "action-flow":
      return <Unavailable what={what} because={notYet("drawing an Action Flow View")} />;
    case "state-transition":
      // OD-03: this view is listed in the mockup and not yet designed.
      return <Unavailable what={what} because={notYet("drawing a State Transition View")} />;
    case "sequence":
      return <Unavailable what={what} because={notYet("drawing a Sequence View")} />;
    case "grid":
      // Built last, per the plan, once its model/ functions exist.
      return <Unavailable what={what} because={notYet("drawing a Grid View")} />;
    case "geometry":
      return <Unavailable what={what} because={notYet("drawing a Geometry View")} />;
    case "browser":
      return <Unavailable what={what} because={notYet("drawing a Browser View")} />;
    default:
      return assertNever(kind);
  }
}

function notYet(capability: string): Readonly<{ kind: "not-implemented"; capability: string }> {
  return { kind: "not-implemented", capability };
}
