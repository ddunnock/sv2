// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { PARSE_CODES } from "./diagnostic";
import { VIEW_KINDS, ViewExposeSchema, ViewKindSchema, ViewSummarySchema } from "./view";

// The views are the mockup's Views list (SCR-01): "GV ThermalControl",
// "IV thermalSubsystem", "AF regulate", "ST HeaterModes", "GR Requirements".

const THERMAL_CONTROL = {
  kind: "resolved",
  target: { kind: "petname", id: "cedar-harbor-208" },
  name: "ThermalControl",
} as const;

const EXPOSE_PACKAGE = { kind: "namespace", target: THERMAL_CONTROL, isRecursive: false } as const;

const GENERAL_VIEW = {
  id: "willow-beacon-551",
  name: "thermalOverview",
  kind: "general",
  exposes: [EXPOSE_PACKAGE],
} as const;

describe("ViewKindSchema", () => {
  test.each([...VIEW_KINDS])("%s is a view kind", (value) => {
    expect(ViewKindSchema.safeParse(value).success).toBe(true);
  });

  test("the union is all eight of clause 9.2.20.2, in clause order", () => {
    // .1 ActionFlowView through .8 StateTransitionView. A ninth, or a missing
    // one, is a change to the specification this cites, not to the code.
    expect([...VIEW_KINDS]).toEqual([
      "action-flow",
      "browser",
      "general",
      "geometry",
      "grid",
      "interconnection",
      "sequence",
      "state-transition",
    ]);
  });

  test("state-transition is a kind although OD-03 leaves it undesigned", () => {
    expect(ViewKindSchema.safeParse("state-transition").success).toBe(true);
  });

  test.each([
    ["the library definition name", "GeneralView"],
    ["the mockup's badge", "GV"],
    ["a SysML v1 diagram", "bdd"],
    ["a plausible name that is not a standard view", "table"],
    ["empty", ""],
  ])("%s is not a view kind", (_name, value) => {
    expect(ViewKindSchema.safeParse(value).success).toBe(false);
  });
});

describe("ViewExposeSchema", () => {
  test.each([
    ["a namespace expose", EXPOSE_PACKAGE],
    ["a recursive namespace expose, the ::** form", { ...EXPOSE_PACKAGE, isRecursive: true }],
    ["a membership expose", { ...EXPOSE_PACKAGE, kind: "membership" }],
  ])("%s parses", (_name, value) => {
    expect(ViewExposeSchema.safeParse(value).success).toBe(true);
  });

  test("an expose of something that does not resolve still parses (ADR-0002)", () => {
    const unresolved = {
      ...EXPOSE_PACKAGE,
      target: { kind: "unresolved", written: "HeaterModes", code: PARSE_CODES.expected },
    };
    expect(ViewExposeSchema.safeParse(unresolved).success).toBe(true);
  });

  test.each([
    // Both are fixed by the metamodel for every Expose; a field that can only
    // hold one value can only be wrong.
    ["a visibility", { ...EXPOSE_PACKAGE, visibility: "protected" }],
    ["isImportAll", { ...EXPOSE_PACKAGE, isImportAll: true }],
    ["the metaclass name instead of the kind", { ...EXPOSE_PACKAGE, kind: "NamespaceExpose" }],
    ["no isRecursive", { kind: "namespace", target: THERMAL_CONTROL }],
  ])("%s is not an expose", (_name, value) => {
    expect(ViewExposeSchema.safeParse(value).success).toBe(false);
  });
});

describe("ViewSummarySchema", () => {
  test("the mockup's General View of ThermalControl parses", () => {
    expect(ViewSummarySchema.safeParse(GENERAL_VIEW).success).toBe(true);
  });

  test("a view whose definition specializes no standard view is still a view", () => {
    expect(ViewSummarySchema.safeParse({ ...GENERAL_VIEW, kind: null }).success).toBe(true);
  });

  test("an unnamed view exposing nothing is still a view", () => {
    expect(ViewSummarySchema.safeParse({ ...GENERAL_VIEW, name: null, exposes: [] }).success).toBe(
      true,
    );
  });

  test.each([
    ["an id that is not a petname", { ...GENERAL_VIEW, id: "ThermalControl::thermalOverview" }],
    ["an absent kind, which must be null (§4.2)", (({ kind: _k, ...rest }) => rest)(GENERAL_VIEW)],
    ["a kind outside the eight", { ...GENERAL_VIEW, kind: "table" }],
    ["layout in the summary, which is the sidecar's (ADR-0017)", { ...GENERAL_VIEW, layout: [] }],
    ["one expose rather than a list", { ...GENERAL_VIEW, exposes: EXPOSE_PACKAGE }],
  ])("%s is not a view", (_name, value) => {
    expect(ViewSummarySchema.safeParse(value).success).toBe(false);
  });
});
