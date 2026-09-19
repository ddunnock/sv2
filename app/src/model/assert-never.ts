// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The end of every exhaustive switch (STD-004-TS §4.5).
 *
 * `tsc` rejects a call whose argument is not `never`, so a switch that ends in
 * `assertNever` stops compiling the day its union gains a variant. Reaching it
 * at run time is a defect — a value that bypassed its schema — so it throws
 * (§7.2).
 *
 * THE MESSAGE CARRIES THE DISCRIMINANT, NOT THE VALUE. §4.5's example
 * stringifies the whole value, but §9.3 forbids model text in an error message,
 * and an unhandled `ElementRef` could carry what the author wrote. The `kind` or
 * `status` field is vocabulary, not model text, and it is what a reader needs
 * to find the switch that missed it.
 */

/** Throws for a variant no case handled. Its parameter type is what makes the switch exhaustive. */
export function assertNever(value: never): never {
  throw new Error(`unhandled variant: ${discriminantOf(value)}`);
}

/** The value's `kind` or `status`, if it has a string one; never anything else about it. */
function discriminantOf(value: unknown): string {
  if (typeof value === "object" && value !== null) {
    for (const key of ["kind", "status"] as const) {
      const tag: unknown = Reflect.get(value, key);
      if (typeof tag === "string") {
        return `${key}=${tag}`;
      }
    }
  }
  return "no discriminant";
}
