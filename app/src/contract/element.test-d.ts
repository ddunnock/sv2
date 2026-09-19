// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Type tests for the element contract (§11 rule 11).
 *
 * Never run; `tsc` checks them. ADR-0002 asks that "the type system make
 * [handling partially resolved elements] unavoidable rather than optional",
 * and that is a claim about types, so it is proved here rather than in a value
 * test — a union that had quietly widened to one object with optional fields
 * would still parse every fixture in `element.test.ts`.
 */

import type { ElementDetail, ElementFacet, ElementRef, FeatureOrigin } from "./element";
import type { ElementHandle, ElementId } from "./element-id";

declare const ref: ElementRef;
declare const facet: ElementFacet;
declare const origin: FeatureOrigin;
declare const detail: ElementDetail;
declare const plain: string;
declare const id: ElementId;

/** ADR-0002: an unresolved reference is not representable as a resolved one. */
// @ts-expect-error the target exists only on the resolved arm; narrow first
export const noUncheckedTarget: ElementHandle = ref.target;

/** Narrowing gives the target. */
export const narrowed: ElementHandle | undefined = ref.kind === "resolved" ? ref.target : undefined;

/** And the other direction: what was written exists only when it did not resolve. */
// @ts-expect-error `written` is on the unresolved arm only
export const noUncheckedWritten: string = ref.written;

/**
 * A reference cannot be both. The id is a minted one, so the only thing wrong
 * here is the code. Each directive sits on the offending property's line
 * because that is where tsc reports, and Biome is free to re-wrap the object.
 */
export const notBoth: ElementRef = {
  kind: "resolved",
  target: { kind: "petname", id },
  name: null,
  // @ts-expect-error a resolved reference has no diagnostic code
  code: plain,
};

/** A handle whose id is an unminted string is not a target, through the ref as well. */
export const noUnmintedTarget: ElementRef = {
  kind: "resolved",
  // @ts-expect-error the id must come from ElementIdSchema (§4.4)
  target: { kind: "petname", id: plain },
  name: null,
};

/** Only a feature facet has types; an element or plain type does not. */
// @ts-expect-error narrow to kind "feature" before reading types
export const noUncheckedTypes: readonly ElementRef[] = facet.types;

/** Only an inherited origin names where it came from. */
// @ts-expect-error narrow to kind "inherited" before reading from
export const noUncheckedFrom: ElementHandle = origin.from;

/** The detail is readonly all the way down; the sidebar cannot edit the model (ADR-0001). */
// @ts-expect-error ElementDetail is Readonly
detail.documentation = [];

// @ts-expect-error its lists are readonly too
detail.ownedFeatures.push(detail.ownedFeatures[0]);
