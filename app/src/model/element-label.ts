// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * How the Specification sidebar words an element (IX-05).
 *
 * Presentation only: the wire carries metaclasses, directions and bounds as
 * written (`contract/element.ts`), and this turns them into the mockup's
 * words — «part def», `attr`, "Satisfied by", `[0..*]`.
 *
 * PARTIAL MAPS THAT FALL BACK TO THE METACLASS. The metaclass set has 176
 * members and the sidebar has words for a handful. An unmapped metaclass is
 * shown by its own name, which is accurate and merely plain — never a guess
 * at a keyword it might have.
 */

import type {
  Metaclass,
  Multiplicity,
  MultiplicityBound,
  RelationshipRow,
} from "@/contract/element";

import { assertNever } from "./assert-never";

/** The textual-notation keyword the header shows in guillemets: `part def`. */
const KEYWORDS: Readonly<Record<string, string>> = {
  Package: "package",
  PartDefinition: "part def",
  PartUsage: "part",
  PortDefinition: "port def",
  PortUsage: "port",
  AttributeDefinition: "attribute def",
  AttributeUsage: "attribute",
  ActionDefinition: "action def",
  ActionUsage: "action",
  RequirementDefinition: "requirement def",
  RequirementUsage: "requirement",
  ItemDefinition: "item def",
  ItemUsage: "item",
  ViewDefinition: "view def",
  ViewUsage: "view",
};

/** The keyword for `metaclass`, or the metaclass itself when there is no mapping. */
export function keyword(metaclass: Metaclass): string {
  return KEYWORDS[metaclass] ?? metaclass;
}

/** The short kind in the Owned features table, as the mockup abbreviates it. */
const FEATURE_KINDS: Readonly<Record<string, string>> = {
  AttributeUsage: "attr",
  PortUsage: "port",
  PartUsage: "part",
  ItemUsage: "item",
  ActionUsage: "action",
  PerformActionUsage: "action",
  ReferenceUsage: "ref",
  ConstraintUsage: "constr",
  RequirementUsage: "req",
};

/** The Owned features "Kind" column for `metaclass`. */
export function featureKind(metaclass: Metaclass): string {
  return FEATURE_KINDS[metaclass] ?? metaclass;
}

/** How a relationship reads from each end: from its source, and from its target. */
const RELATIONSHIPS: Readonly<Record<string, readonly [string, string]>> = {
  SatisfyRequirementUsage: ["Satisfies", "Satisfied by"],
  FeatureTyping: ["Typed by", "Usages"],
  Subclassification: ["Specializes", "Specialized by"],
  Redefinition: ["Redefines", "Redefined by"],
  PerformActionUsage: ["Performs", "Performed by"],
  OwningMembership: ["Owns", "Owned by"],
};

/** The Relationships row's label, from this element's end of it. */
export function relationshipLabel(row: Pick<RelationshipRow, "metaclass" | "direction">): string {
  const words = RELATIONSHIPS[row.metaclass];
  if (words !== undefined) {
    return row.direction === "outgoing" ? words[0] : words[1];
  }
  return `${row.metaclass} ${row.direction === "outgoing" ? "→" : "←"}`;
}

/** A multiplicity as written: `1`, `0..*`, `2..n`; an em dash when none was written. */
export function formatMultiplicity(multiplicity: Multiplicity | null): string {
  if (multiplicity === null) {
    return "—";
  }
  const upper = formatBound(multiplicity.upper);
  return multiplicity.lower === null ? upper : `${formatBound(multiplicity.lower)}..${upper}`;
}

function formatBound(bound: MultiplicityBound): string {
  switch (bound.kind) {
    case "literal":
      return String(bound.value);
    case "unbounded":
      return "*";
    case "expression":
      return bound.text;
    default:
      return assertNever(bound);
  }
}
