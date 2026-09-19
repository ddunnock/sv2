// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { PARSE_CODES } from "./diagnostic";
import {
  ElementDetailSchema,
  ElementFacetSchema,
  ElementRefSchema,
  FeatureOriginSchema,
  FeatureRowSchema,
  MetaclassSchema,
  MultiplicitySchema,
  RelationshipRowSchema,
  VISIBILITIES,
  VisibilitySchema,
} from "./element";

// The sample model is the mockup's (docs/design/sysml-workbench-mockup), so
// the shapes below are ones its Specification sidebar actually draws.

const HEATER = { kind: "petname", id: "maple-sunrise-314" } as const;
const PACKAGE = { kind: "petname", id: "cedar-harbor-208" } as const;
const CONTROLLER = { kind: "petname", id: "river-lantern-117" } as const;

/**
 * Shaped like a KerML name-based UUID, not a real one. `z.uuid()` checks the
 * RFC 9562 layout, which is all the handle claims to check.
 */
const LIBRARY_PART = { kind: "library", uuid: "e5b6a8f0-3d2c-5a1b-9c4d-0123456789ab" } as const;

/** `Heater`'s `state : HeaterState`, the mockup's deliberate unresolved type. */
const UNRESOLVED = {
  kind: "unresolved",
  written: "HeaterState",
  code: PARSE_CODES.expected,
} as const;

const RESOLVED = { kind: "resolved", target: CONTROLLER, name: "ThermalController" } as const;

const ONE = { lower: null, upper: { kind: "literal", value: 1 } } as const;

const FEATURE_ROW = {
  handle: { kind: "petname", id: "amber-meadow-402" },
  metaclass: "AttributeUsage",
  name: "state",
  types: [UNRESOLVED],
  multiplicity: ONE,
  origin: { kind: "owned" },
} as const;

const IMPLIED_SPECIALIZATION = {
  handle: { kind: "derived", hash: "9f2c1ab04e" },
  metaclass: "Subclassification",
  direction: "outgoing",
  other: { kind: "resolved", target: LIBRARY_PART, name: "Part" },
  isImplied: true,
} as const;

const HEATER_DETAIL = {
  summary: {
    handle: HEATER,
    metaclass: "PartDefinition",
    name: "Heater",
    shortName: null,
    qualifiedName: "ThermalControl::Heater",
  },
  facet: { kind: "type", isAbstract: false, specializes: [IMPLIED_SPECIALIZATION] },
  owner: { kind: "resolved", target: PACKAGE, name: "ThermalControl" },
  visibility: "public",
  location: { file: "model/ThermalControl.sysml", span: { start: 402, end: 561 } },
  documentation: ["Resistive film heater."],
  ownedFeatures: [FEATURE_ROW],
  relationships: [],
  diagnostics: [
    {
      code: PARSE_CODES.expected,
      severity: "error",
      span: { start: 480, end: 491 },
      message: "cannot resolve type `HeaterState`",
    },
  ],
} as const;

describe("MetaclassSchema", () => {
  test.each(["PartDefinition", "ConjugatedPortDefinition", "Subclassification", "Feature"])(
    "%s is a metaclass name",
    (name) => {
      expect(MetaclassSchema.safeParse(name).success).toBe(true);
    },
  );

  test.each([
    ["lowercase first", "partDefinition"],
    ["a qualified name", "SysML::PartDefinition"],
    ["the mockup's abbreviation", "attr"],
    ["a keyword", "part def"],
    ["a digit", "Part2"],
    ["empty", ""],
  ])("%s is not a metaclass name", (_name, value) => {
    expect(MetaclassSchema.safeParse(value).success).toBe(false);
  });
});

describe("VisibilitySchema", () => {
  test.each([...VISIBILITIES])("%s is a visibility", (value) => {
    expect(VisibilitySchema.safeParse(value).success).toBe(true);
  });

  test.each(["Public", "package", "internal", ""])("%s is not a visibility", (value) => {
    expect(VisibilitySchema.safeParse(value).success).toBe(false);
  });
});

describe("ElementRefSchema", () => {
  test("a resolved reference parses", () => {
    expect(ElementRefSchema.safeParse(RESOLVED).success).toBe(true);
  });

  test("a resolved reference to an unnamed element parses, because Element::name is [0..1]", () => {
    expect(ElementRefSchema.safeParse({ ...RESOLVED, name: null }).success).toBe(true);
  });

  test("a conjugated port type is a resolved reference whose name carries the ~", () => {
    const conjugated = {
      kind: "resolved",
      target: { kind: "derived", hash: "c0ffee42" },
      name: "~TempPort",
    };
    expect(ElementRefSchema.safeParse(conjugated).success).toBe(true);
  });

  test("an unresolved reference parses (ADR-0002 admits it)", () => {
    expect(ElementRefSchema.safeParse(UNRESOLVED).success).toBe(true);
  });

  test.each([
    // ADR-0002: an unresolved reference must not be representable as a
    // resolved one. Each of these is an attempt to be both at once.
    ["an unresolved reference with a target", { ...UNRESOLVED, target: CONTROLLER }],
    ["a resolved reference with a code", { ...RESOLVED, code: PARSE_CODES.expected }],
    ["a resolved reference with no target", { kind: "resolved", name: "ThermalController" }],
    ["an unresolved reference with no code", { kind: "unresolved", written: "HeaterState" }],
    ["an unresolved reference with a bad code", { ...UNRESOLVED, code: "unresolved" }],
    [
      "a missing name, which must be null rather than absent (§4.2)",
      { kind: "resolved", target: CONTROLLER },
    ],
    ["an unknown kind", { kind: "partial", written: "HeaterState" }],
  ])("%s is not a reference", (_name, value) => {
    expect(ElementRefSchema.safeParse(value).success).toBe(false);
  });
});

describe("MultiplicitySchema", () => {
  test.each([
    ["[1]", ONE],
    ["[0..*]", { lower: { kind: "literal", value: 0 }, upper: { kind: "unbounded" } }],
    ["[2..n]", { lower: { kind: "literal", value: 2 }, upper: { kind: "expression", text: "n" } }],
    ["the u32 maximum", { lower: null, upper: { kind: "literal", value: 0xffff_ffff } }],
  ])("%s parses", (_name, value) => {
    expect(MultiplicitySchema.safeParse(value).success).toBe(true);
  });

  test.each([
    ["no upper bound", { lower: null }],
    ["an absent lower bound, which must be null (§4.2)", { upper: { kind: "unbounded" } }],
    ["a negative bound", { lower: null, upper: { kind: "literal", value: -1 } }],
    ["a fractional bound", { lower: null, upper: { kind: "literal", value: 1.5 } }],
    ["a bound above u32 (§4.2)", { lower: null, upper: { kind: "literal", value: 2 ** 32 } }],
    ["a bound as text", { lower: null, upper: { kind: "literal", value: "1" } }],
    ["an unbounded bound with a value", { lower: null, upper: { kind: "unbounded", value: 1 } }],
  ])("%s is not a multiplicity", (_name, value) => {
    expect(MultiplicitySchema.safeParse(value).success).toBe(false);
  });
});

describe("FeatureOriginSchema", () => {
  test("owned parses", () => {
    expect(FeatureOriginSchema.safeParse({ kind: "owned" }).success).toBe(true);
  });

  test("inherited names where from, so the row can link there (OD-04)", () => {
    expect(FeatureOriginSchema.safeParse({ kind: "inherited", from: CONTROLLER }).success).toBe(
      true,
    );
  });

  test.each([
    ["inherited with nowhere to have come from", { kind: "inherited" }],
    ["owned with a source", { kind: "owned", from: CONTROLLER }],
  ])("%s is not an origin", (_name, value) => {
    expect(FeatureOriginSchema.safeParse(value).success).toBe(false);
  });
});

describe("FeatureRowSchema", () => {
  test("a row with an unresolved type parses, and keeps what was written", () => {
    const result = FeatureRowSchema.safeParse(FEATURE_ROW);
    expect(result.success).toBe(true);
    expect(result.success && result.data.types[0]).toEqual(UNRESOLVED);
  });

  test("an untyped feature is legal, and is not the same as an unresolved one", () => {
    expect(FeatureRowSchema.safeParse({ ...FEATURE_ROW, types: [] }).success).toBe(true);
  });

  test("several types are legal, because Feature::type is [0..*]", () => {
    expect(
      FeatureRowSchema.safeParse({ ...FEATURE_ROW, types: [RESOLVED, UNRESOLVED] }).success,
    ).toBe(true);
  });

  test.each([
    ["no origin", (({ origin: _o, ...rest }) => rest)(FEATURE_ROW)],
    ["a presentation label for a metaclass", { ...FEATURE_ROW, metaclass: "attr" }],
    ["a single type rather than a list", { ...FEATURE_ROW, types: UNRESOLVED }],
    ["an extra key", { ...FEATURE_ROW, value: "0.5" }],
  ])("%s is not a feature row", (_name, value) => {
    expect(FeatureRowSchema.safeParse(value).success).toBe(false);
  });
});

describe("RelationshipRowSchema", () => {
  test("an implied specialization parses", () => {
    expect(RelationshipRowSchema.safeParse(IMPLIED_SPECIALIZATION).success).toBe(true);
  });

  test("the far end may be unresolved (ADR-0002)", () => {
    expect(
      RelationshipRowSchema.safeParse({ ...IMPLIED_SPECIALIZATION, other: UNRESOLVED }).success,
    ).toBe(true);
  });

  test.each([
    [
      "a presentation label for a direction",
      { ...IMPLIED_SPECIALIZATION, direction: "Satisfied by" },
    ],
    ["no isImplied", (({ isImplied: _i, ...rest }) => rest)(IMPLIED_SPECIALIZATION)],
  ])("%s is not a relationship row", (_name, value) => {
    expect(RelationshipRowSchema.safeParse(value).success).toBe(false);
  });
});

describe("ElementFacetSchema", () => {
  const featureFacet = {
    kind: "feature",
    isAbstract: false,
    specializes: [],
    types: [RESOLVED],
    multiplicity: ONE,
    isComposite: true,
    redefines: [],
  } as const;

  test.each([
    ["an element that is not a type", { kind: "element" }],
    ["a type", HEATER_DETAIL.facet],
    ["a feature", featureFacet],
  ])("%s parses", (_name, value) => {
    expect(ElementFacetSchema.safeParse(value).success).toBe(true);
  });

  test.each([
    // A Feature is a Type, so a feature facet without the type fields is
    // the representable-and-meaningless state the union exists to exclude.
    ["a feature without the type fields", (({ isAbstract: _a, ...rest }) => rest)(featureFacet)],
    // Usage::isReference is derived as isComposite = false; carrying both
    // would be two values that can disagree.
    ["a feature carrying isReference", { ...featureFacet, isReference: false }],
    ["a plain element with type fields", { kind: "element", isAbstract: false }],
  ])("%s is not a facet", (_name, value) => {
    expect(ElementFacetSchema.safeParse(value).success).toBe(false);
  });
});

describe("ElementDetailSchema", () => {
  test("the mockup's Heater parses, diagnostic and all", () => {
    const result = ElementDetailSchema.safeParse(HEATER_DETAIL);
    expect(result.success).toBe(true);
    expect(result.success && result.data.diagnostics).toHaveLength(1);
  });

  test("a root namespace has no owner and no visibility", () => {
    expect(
      ElementDetailSchema.safeParse({ ...HEATER_DETAIL, owner: null, visibility: null }).success,
    ).toBe(true);
  });

  test("a library element has no location in the workspace", () => {
    expect(ElementDetailSchema.safeParse({ ...HEATER_DETAIL, location: null }).success).toBe(true);
  });

  test.each([
    [
      "an absent owner, which must be null (§4.2)",
      (({ owner: _o, ...rest }) => rest)(HEATER_DETAIL),
    ],
    [
      "an empty file path",
      { ...HEATER_DETAIL, location: { file: "", span: { start: 0, end: 1 } } },
    ],
    ["documentation as one string", { ...HEATER_DETAIL, documentation: "Resistive film heater." }],
    [
      "a staged edit buffer, which one-document editing has no place for",
      { ...HEATER_DETAIL, pendingSource: "" },
    ],
  ])("%s is not an element detail", (_name, value) => {
    expect(ElementDetailSchema.safeParse(value).success).toBe(false);
  });
});
