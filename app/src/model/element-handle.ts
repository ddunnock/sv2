// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Operations on `ElementHandle` that every layer needs and none should repeat.
 *
 * WHAT MAY BE LOGGED ABOUT AN ELEMENT. §9.3 forbids model text in any report —
 * no name, no qualified name — and says the petname is the log-safe handle.
 * `logLabel` extends that to every arm, because a report about a derived
 * relationship or a library element has to name its subject too (§9.4). None
 * of the arms carries model meaning: ADR-0016 designed petnames to be
 * speakable, a derived handle is a hash, a library UUID names a public standard
 * element, and a span is two counts. So every handle has a label, and the
 * function is total rather than a predicate that sometimes says no.
 */

import type { ElementHandle } from "@/contract/element-id";
import { assertNever } from "./assert-never";

/**
 * A label for `handle` that is safe to write to a log (§9.3) and says what the
 * report is about (§9.4).
 *
 * A membership is labelled `<owned-ID>/m`, the derivation ADR-0016 gives for
 * it. The other non-petname arms are prefixed so a reader can tell which
 * namespace a string came from.
 */
export function logLabel(handle: ElementHandle): string {
  switch (handle.kind) {
    case "petname":
      return handle.id;
    case "membership":
      return `${handle.of}/m`;
    case "derived":
      return `derived:${handle.hash}`;
    case "library":
      return `library:${handle.uuid}`;
    case "unidentified":
      return `unidentified@${handle.span.start}-${handle.span.end}`;
    default:
      return assertNever(handle);
  }
}

/**
 * True when two handles address the same element.
 *
 * Handles are values, and two fetched separately are different objects, so
 * `===` compares the wrong thing. Two `unidentified` handles compare by span,
 * which is only meaningful within one file — acceptable because the arm exists
 * only while a workspace is opening (ADR-0020), when nothing is selected.
 */
export function sameHandle(a: ElementHandle, b: ElementHandle): boolean {
  switch (a.kind) {
    case "petname":
      return b.kind === "petname" && a.id === b.id;
    case "membership":
      return b.kind === "membership" && a.of === b.of;
    case "derived":
      return b.kind === "derived" && a.hash === b.hash;
    case "library":
      return b.kind === "library" && a.uuid === b.uuid;
    case "unidentified":
      return (
        b.kind === "unidentified" && a.span.start === b.span.start && a.span.end === b.span.end
      );
    default:
      return assertNever(a);
  }
}
