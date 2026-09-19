// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { VIEW_KINDS, type ViewSummary, ViewSummarySchema } from "@/contract/view";

import { kindLabel, viewCaption } from "./view-label";

function view(raw: unknown): ViewSummary {
  const parsed = ViewSummarySchema.safeParse(raw);
  if (!parsed.success) {
    throw new Error("fixture does not satisfy ViewSummarySchema");
  }
  return parsed.data;
}

const BASE = { id: "willow-beacon-551", name: "thermalOverview", kind: "general", exposes: [] };

describe("kindLabel", () => {
  test.each([
    ["general", "GV"],
    ["interconnection", "IV"],
    ["action-flow", "AF"],
    ["state-transition", "ST"],
    ["grid", "GR"],
  ] as const)("%s carries the mockup's badge, %s", (kind, badge) => {
    expect(kindLabel(kind).badge).toBe(badge);
  });

  test("every kind has a distinct badge and name", () => {
    const labels = VIEW_KINDS.map(kindLabel);
    expect(new Set(labels.map((label) => label.badge)).size).toBe(VIEW_KINDS.length);
    expect(new Set(labels.map((label) => label.name)).size).toBe(VIEW_KINDS.length);
  });

  test("a view of no standard kind is still labelled", () => {
    expect(kindLabel(null)).toEqual({ name: "View", badge: "V" });
  });
});

describe("viewCaption", () => {
  test("is the first exposed element, as the mockup names views", () => {
    const exposes = [
      {
        kind: "namespace",
        target: {
          kind: "resolved",
          target: { kind: "petname", id: "cedar-harbor-208" },
          name: "ThermalControl",
        },
        isRecursive: false,
      },
    ];
    expect(viewCaption(view({ ...BASE, exposes }))).toBe("ThermalControl");
  });

  test("an unresolved expose shows what the author wrote (ADR-0002)", () => {
    const exposes = [
      {
        kind: "membership",
        target: { kind: "unresolved", written: "HeaterModes", code: "RES-UNRESOLVED-NAME" },
        isRecursive: false,
      },
    ];
    expect(viewCaption(view({ ...BASE, exposes }))).toBe("HeaterModes");
  });

  test("a view exposing nothing falls back to its own name, then to a placeholder", () => {
    expect(viewCaption(view(BASE))).toBe("thermalOverview");
    expect(viewCaption(view({ ...BASE, name: null }))).toBe("(unnamed view)");
  });
});
