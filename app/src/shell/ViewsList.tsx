// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The Views list at the foot of the navigator (UI-03).
 *
 * Each view is a button named by its kind and what it exposes — "General
 * View, ThermalControl" to a screen reader, "GV ThermalControl" on screen —
 * and pressing it opens the view in a tab. The list comes from the `views`
 * query, rendered through `AnswerView`, so an unavailable answer or a failed
 * read shows here the same way it does everywhere.
 */

import type { ViewSummary } from "@/contract/view";
import { kindLabel, viewCaption } from "@/model/view-label";

import { AnswerView } from "./AnswerView";
import { useAnswer } from "./services";

/** Props for `ViewsList`. */
export type ViewsListProps = Readonly<{ onOpen: (view: ViewSummary) => void }>;

/** Every view in the workspace, each opening in a tab. */
export function ViewsList({ onOpen }: ViewsListProps): React.JSX.Element {
  const views = useAnswer((queries) => queries.views(), "views");
  return (
    <section aria-labelledby="views-heading" className="border-line border-t">
      <h2 id="views-heading" className="px-3 pt-2 font-semibold text-meta text-muted uppercase">
        Views
      </h2>
      <AnswerView query={views} what="The Views list">
        {(data) =>
          data.length === 0 ? (
            <p className="px-3 py-2 text-muted">This workspace has no views.</p>
          ) : (
            <ul className="py-1">
              {data.map((view) => {
                const { name, badge } = kindLabel(view.kind);
                const caption = viewCaption(view);
                return (
                  <li key={view.id}>
                    <button
                      type="button"
                      aria-label={`${name}, ${caption}`}
                      onClick={() => {
                        onOpen(view);
                      }}
                      className="flex h-6 w-full items-center gap-2 px-3 text-left text-fg hover:bg-hover"
                    >
                      <span aria-hidden="true" className="w-5 font-mono text-faint text-micro">
                        {badge}
                      </span>
                      <span className="truncate">{caption}</span>
                    </button>
                  </li>
                );
              })}
            </ul>
          )
        }
      </AnswerView>
    </section>
  );
}
