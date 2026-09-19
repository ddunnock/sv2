// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * What an island shows in place of an answer it does not have.
 *
 * It says what is true and nothing else. Never sample data, never a greyed-out
 * mock, never a spinner: a spinner is a claim that something is loading, and
 * for every reason but one, nothing is. **Not even an empty dotted canvas** for
 * a diagram — that reads as "the diagram works and this model is empty", which
 * is false.
 *
 * The one reason that is in progress, `opening`, is a live status region, so
 * assistive technology hears when it changes without being interrupted. Every
 * other reason is static text, because it will not change on its own.
 *
 * `what` names the thing that is unavailable ("General View"); `because` is the
 * core's reason, carried through from the answer rather than restated here.
 */

import type { UnavailableReason } from "@/contract/availability";
import { assertNever } from "@/model/assert-never";

/** Props for `Unavailable`. */
export type UnavailableProps = Readonly<{
  /** What cannot be shown: "General View", "Specification". */
  what: string;
  because: UnavailableReason;
}>;

/** The sentence for each reason. `what` is the subject of every one. */
function explain(what: string, because: UnavailableReason): string {
  switch (because.kind) {
    case "not-implemented":
      return `${what} is not available yet: sv2 does not yet support ${because.capability}.`;
    case "no-workspace":
      return `${what} needs an open workspace.`;
    case "opening":
      return `${what} will be available when the workspace has finished opening.`;
    case "read-only":
      return `${what} is not available because this workspace is read-only.`;
    case "not-found":
      return `${what} is not available: what it refers to is no longer in the model.`;
    default:
      return assertNever(because);
  }
}

/** A plain statement of why an answer is missing. */
export function Unavailable({ what, because }: UnavailableProps): React.JSX.Element {
  const text = explain(what, because);
  return because.kind === "opening" ? (
    <p role="status" className="p-4 text-muted text-ui">
      {text}
    </p>
  ) : (
    <p className="p-4 text-muted text-ui">{text}</p>
  );
}
