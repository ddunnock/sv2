// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Why a call across the boundary produced no readable reply (§7.1).
 *
 * Every arm is a defect or an outage, not an answer: a correct reply that says
 * the core cannot answer is `unavailable`, inside a successful `Result`. These
 * are what `diagnostics/` reports.
 *
 * NOTHING HERE CARRIES WHAT WAS RECEIVED. §7.1: a contract error records the
 * boundary and, per issue, its path and Zod issue code — never the input,
 * which may be model text (§9.3). A rejected command carries only its name for
 * the same reason: its error payload is unparsed backend output.
 */

/** One way a reply failed its schema: where, and which check. */
export type ContractIssue = Readonly<{ path: readonly (string | number)[]; code: string }>;

/** Why a call produced no readable reply. */
export type IpcError =
  | Readonly<{ kind: "unreachable"; command: string }>
  | Readonly<{ kind: "rejected"; command: string }>
  | Readonly<{ kind: "contract"; boundary: string; issues: readonly ContractIssue[] }>;

/**
 * The part of a schema failure this module reads. Stated structurally rather
 * than imported from Zod, because `ipc/` may import only `@tauri-apps/api`
 * from outside the project (§2.1); a `ZodError` satisfies it.
 */
export type SchemaFailure = Readonly<{
  issues: readonly Readonly<{ path: readonly PropertyKey[]; code: string }>[];
}>;

/**
 * A contract error for `boundary`, keeping each issue's path and code only.
 *
 * Zod puts the received value in an issue only when `reportInput` is set, and
 * it never is; copying two fields rather than the issue makes that hold even if
 * it were.
 */
export function contractError(boundary: string, error: SchemaFailure): IpcError {
  return {
    kind: "contract",
    boundary,
    issues: error.issues.map((issue) => ({
      path: issue.path.map((key) => (typeof key === "symbol" ? "<symbol>" : key)),
      code: issue.code,
    })),
  };
}
