// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { type Multiplicity, MultiplicitySchema } from "@/contract/element";

import { featureKind, formatMultiplicity, keyword, relationshipLabel } from "./element-label";

function multiplicity(raw: unknown): Multiplicity {
  const parsed = MultiplicitySchema.safeParse(raw);
  if (!parsed.success) {
    throw new Error("fixture does not satisfy MultiplicitySchema");
  }
  return parsed.data;
}

describe("keyword", () => {
  test.each([
    ["PartDefinition", "part def"],
    ["RequirementDefinition", "requirement def"],
    ["PortDefinition", "port def"],
  ])("%s is «%s», as the mockup's headers read", (metaclass, word) => {
    expect(keyword(metaclass)).toBe(word);
  });

  test("an unmapped metaclass is shown as itself, never guessed at", () => {
    expect(keyword("ConjugatedPortDefinition")).toBe("ConjugatedPortDefinition");
  });
});

describe("featureKind", () => {
  test.each([
    ["AttributeUsage", "attr"],
    ["PortUsage", "port"],
    ["PerformActionUsage", "action"],
  ])("%s is %s, as the mockup's Owned features table abbreviates", (metaclass, kind) => {
    expect(featureKind(metaclass)).toBe(kind);
  });

  test("an unmapped metaclass is shown as itself", () => {
    expect(featureKind("EventOccurrenceUsage")).toBe("EventOccurrenceUsage");
  });
});

describe("relationshipLabel", () => {
  test("one relationship reads differently from each end", () => {
    expect(relationshipLabel({ metaclass: "SatisfyRequirementUsage", direction: "outgoing" })).toBe(
      "Satisfies",
    );
    expect(relationshipLabel({ metaclass: "SatisfyRequirementUsage", direction: "incoming" })).toBe(
      "Satisfied by",
    );
  });

  test("an unmapped relationship shows its metaclass and direction", () => {
    expect(relationshipLabel({ metaclass: "Dependency", direction: "incoming" })).toBe(
      "Dependency ←",
    );
  });
});

describe("formatMultiplicity", () => {
  test.each([
    ["[1]", { lower: null, upper: { kind: "literal", value: 1 } }, "1"],
    ["[0..*]", { lower: { kind: "literal", value: 0 }, upper: { kind: "unbounded" } }, "0..*"],
    [
      "[2..n]",
      { lower: { kind: "literal", value: 2 }, upper: { kind: "expression", text: "n" } },
      "2..n",
    ],
  ])("%s is written as %s", (_name, raw, text) => {
    expect(formatMultiplicity(multiplicity(raw))).toBe(text);
  });

  test("none written is an em dash, not an invented default", () => {
    expect(formatMultiplicity(null)).toBe("—");
  });
});
