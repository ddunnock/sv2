// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * An answer from the core: the data, or the reason there is none.
 *
 * RUST COUNTERPART: none yet. Every model query returns one of these, so its
 * owner is whichever crate answers queries; the Rust shape is an enum with
 * `#[serde(tag = "kind")]`, per §4.2's union convention.
 *
 * UNAVAILABLE IS AN ANSWER, NOT A FAILURE. `model/result.ts`'s `Result` is for
 * a reply that could not be read — a broken IPC call or a shape that failed its
 * schema, both defects. An `Answer` is a reply that was read correctly and says
 * the core cannot answer this. The two are kept apart because they are handled
 * apart: a defect is reported (§9), an unavailable answer is rendered, by
 * `<Unavailable>`, where the answer would have been.
 *
 * AND IT NEVER GOES AWAY. Today almost every query is `not-implemented`,
 * because `sv2-resolve` is a stub. When it lands, that reason retires query by
 * query, and the others remain the correct permanent answer for their cases.
 * The placeholder path and the production path are one code path, so building
 * the shell against fixtures now builds the error handling it ships with.
 *
 * WHAT IS NOT A REASON, AND WHY:
 * - A file that does not parse. The parser does not fail outright (invariant
 *   3), and ADR-0002 admits everything that parses, carrying its diagnostics;
 *   a broken file keeps its last good subtree. The answer is `ready`, decorated.
 * - An unsupported language. `WorkspaceFile.language` is `sysml` or `kerml`
 *   (ADR-0014), so no query can name a file in a third one.
 *
 * THE SUBJECT IS NOT CARRIED. The caller knows what it asked, and matching a
 * late reply to a request that has since been superseded is `ipc/`'s job,
 * which it has to do for `ready` answers too.
 */

import { z } from "zod";

/**
 * Why the core has no answer.
 *
 * - `not-implemented`: the core does not do this yet. `capability` names what
 *   is missing, so the panel can say what it is waiting for.
 * - `no-workspace`: nothing is open, so there is nothing to ask about.
 * - `opening`: the workspace is being opened and identities allocated. ADR-0020
 *   calls this a loading state, and "the honest thing to render during it is
 *   that the workspace is still opening". It is the one reason that will pass
 *   on its own.
 * - `read-only`: the workspace cannot be written, and ADR-0020 requires that
 *   anything which would persist a handle be "unavailable rather than silently
 *   ineffective".
 * - `not-found`: the subject no longer exists. An edit renamed or deleted what
 *   was selected — the rename that leaves references dangling for seconds is
 *   ADR-0002's own first driver.
 */
export type UnavailableReason =
  | Readonly<{ kind: "not-implemented"; capability: string }>
  | Readonly<{ kind: "no-workspace" }>
  | Readonly<{ kind: "opening" }>
  | Readonly<{ kind: "read-only" }>
  | Readonly<{ kind: "not-found" }>;

const notImplemented = z.strictObject({
  kind: z.literal("not-implemented"),
  capability: z.string().min(1),
});
const noWorkspace = z.strictObject({ kind: z.literal("no-workspace") });
const opening = z.strictObject({ kind: z.literal("opening") });
const readOnly = z.strictObject({ kind: z.literal("read-only") });
const notFound = z.strictObject({ kind: z.literal("not-found") });

/**
 * Why the core has no answer.
 *
 * Closed. A reason the webview does not know has no rendering, and a query
 * whose answer it cannot render is better failed as a contract error — which
 * is reported — than shown as a blank.
 */
export const UnavailableReasonSchema: z.ZodType<UnavailableReason, unknown> = z.discriminatedUnion(
  "kind",
  [notImplemented, noWorkspace, opening, readOnly, notFound],
);

/** An answer from the core: the data, or the reason there is none. */
export type Answer<T> =
  | Readonly<{ kind: "ready"; data: T }>
  | Readonly<{ kind: "unavailable"; reason: UnavailableReason }>;

const unavailableArm = z.strictObject({
  kind: z.literal("unavailable"),
  reason: UnavailableReasonSchema,
});

/**
 * The schema for an answer carrying `data`.
 *
 * A function because `Answer` is generic and every query has its own data
 * schema. `z.union` rather than `z.discriminatedUnion`: the ready arm is built
 * per call around a caller's schema, and a plain union of two arms with
 * distinct `kind` literals rejects exactly what the discriminated one would.
 */
export function answerSchema<T>(data: z.ZodType<T, unknown>): z.ZodType<Answer<T>, unknown> {
  return z.union([z.strictObject({ kind: z.literal("ready"), data }), unavailableArm]);
}
