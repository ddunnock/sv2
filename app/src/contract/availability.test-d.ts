// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Type tests for answers (§11 rule 11).
 *
 * Never run; `tsc` checks them. The claim is that a component cannot render
 * data it does not have: reaching `data` requires first establishing that the
 * answer is ready, and a switch over the reasons that forgets one does not
 * compile — so a new reason arrives as a compile error in `<Unavailable>`,
 * not as a blank panel.
 */

import type { Answer, UnavailableReason } from "./availability";
import type { Workspace } from "./file";

declare const answer: Answer<Workspace>;
declare const reason: UnavailableReason;

/** Data exists only on the ready arm. */
// @ts-expect-error narrow to kind "ready" before reading data
export const unchecked: Workspace = answer.data;

/** Narrowing gives it. */
export const checked: Workspace | undefined = answer.kind === "ready" ? answer.data : undefined;

/** Every reason handled: this is the shape `<Unavailable>` will take. */
function describeReason(r: UnavailableReason): string {
  switch (r.kind) {
    case "not-implemented":
      return r.capability;
    case "no-workspace":
    case "opening":
    case "read-only":
    case "not-found":
      return r.kind;
    default: {
      const unreachable: never = r;
      return unreachable;
    }
  }
}
export const described: string = describeReason(reason);

/** One forgotten: the leftover is not never, so this does not compile. */
function forgetsOpening(r: UnavailableReason): string {
  switch (r.kind) {
    case "not-implemented":
    case "no-workspace":
    case "read-only":
    case "not-found":
      return r.kind;
    default: {
      // @ts-expect-error "opening" is left over, and it is not never
      const unreachable: never = r;
      return unreachable;
    }
  }
}
export const forgotten: string = forgetsOpening(reason);

/** Only not-implemented names a capability. */
// @ts-expect-error narrow to kind "not-implemented" before reading capability
export const noUncheckedCapability: string = reason.capability;
