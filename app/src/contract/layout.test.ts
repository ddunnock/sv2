// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { DurableHandleSchema } from "./element-id";
import { GridUnitSchema, LAYOUT_SCHEMA, RECORD_SOURCES, ViewLayoutSchema } from "./layout";

// The records are ADR-0017's own example, moved onto the wire: camelCase,
// handles for element strings, grouped by kind.

const HEADER = {
  schema: 1,
  view: "amber-lattice-003",
  engine: "orthogonal",
  engineVersion: "0.4.0",
  grid: 8,
} as const;

const PINNED_NODE = {
  element: { kind: "petname", id: "crisp-harbor-042" },
  x: 320,
  y: 176,
  w: 160,
  h: 96,
  src: "pinned",
} as const;

const AUTO_NODE = {
  ...PINNED_NODE,
  element: { kind: "petname", id: "quiet-meadow-117" },
  x: 544,
  src: "auto",
} as const;

const EDGE = {
  element: { kind: "petname", id: "brisk-anchor-008" },
  waypoints: [
    [480, 224],
    [544, 224],
  ],
  src: "auto",
} as const;

const STATE = {
  element: { kind: "petname", id: "crisp-harbor-042" },
  collapsed: false,
  compartments: ["parts", "ports"],
} as const;

const LAYOUT = {
  header: HEADER,
  nodes: [PINNED_NODE, AUTO_NODE],
  edges: [EDGE],
  states: [STATE],
  unknown: [],
} as const;

describe("GridUnitSchema", () => {
  test.each([
    ["zero", 0],
    ["a coordinate from ADR-0017", 320],
    ["a negative coordinate, left of the origin", -48],
    ["the i32 minimum", -(2 ** 31)],
    ["the i32 maximum", 2 ** 31 - 1],
  ])("%s is a grid unit", (_name, value) => {
    const result = GridUnitSchema.safeParse(value);
    expect(result.success).toBe(true);
    expect(Number(result.success && result.data)).toBe(value);
  });

  test.each([
    // R-1: no floating-point values are written.
    ["a fraction", 320.5],
    ["below i32", -(2 ** 31) - 1],
    ["above i32", 2 ** 31],
    ["a string", "320"],
    ["NaN", Number.NaN],
    ["Infinity", Number.POSITIVE_INFINITY],
  ])("%s is not a grid unit", (_name, value) => {
    expect(GridUnitSchema.safeParse(value).success).toBe(false);
  });
});

describe("DurableHandleSchema", () => {
  test.each([
    ["a petname", { kind: "petname", id: "crisp-harbor-042" }],
    ["a membership", { kind: "membership", of: "crisp-harbor-042" }],
    ["an implied relationship, which an edge can be", { kind: "derived", hash: "9f2c1ab04e" }],
    [
      "a library element, which a view can expose",
      { kind: "library", uuid: "e5b6a8f0-3d2c-5a1b-9c4d-0123456789ab" },
    ],
  ])("%s is durable", (_name, value) => {
    expect(DurableHandleSchema.safeParse(value).success).toBe(true);
  });

  test("an unidentified handle is not durable, because a span means nothing outside its file (ADR-0020)", () => {
    const unidentified = { kind: "unidentified", span: { start: 0, end: 12 } };
    expect(DurableHandleSchema.safeParse(unidentified).success).toBe(false);
  });
});

describe("ViewLayoutSchema", () => {
  test("ADR-0017's example parses", () => {
    expect(ViewLayoutSchema.safeParse(LAYOUT).success).toBe(true);
  });

  test("an empty layout parses, because a view with nothing placed yet is legal (DD-4)", () => {
    const empty = { header: HEADER, nodes: [], edges: [], states: [], unknown: [] };
    expect(ViewLayoutSchema.safeParse(empty).success).toBe(true);
  });

  test.each([...RECORD_SOURCES])("src %s parses (R-4)", (src) => {
    expect(
      ViewLayoutSchema.safeParse({ ...LAYOUT, nodes: [{ ...PINNED_NODE, src }] }).success,
    ).toBe(true);
  });

  test("an edge with no waypoints is a straight line, and parses", () => {
    expect(
      ViewLayoutSchema.safeParse({ ...LAYOUT, edges: [{ ...EDGE, waypoints: [] }] }).success,
    ).toBe(true);
  });

  describe("R-6: unknown fields and kinds are carried, not dropped", () => {
    test("an unknown field on a node survives the parse, value and all", () => {
      const node = { ...PINNED_NODE, z_order: 3, label_anchor: { dx: 1, dy: -1 } };
      const result = ViewLayoutSchema.safeParse({ ...LAYOUT, nodes: [node] });
      expect(result.success).toBe(true);
      expect(result.success && result.data.nodes[0]).toMatchObject({
        z_order: 3,
        label_anchor: { dx: 1, dy: -1 },
      });
    });

    test("an unknown field on the header survives", () => {
      const header = { ...HEADER, generated_by: "sv2 0.9.0" };
      const result = ViewLayoutSchema.safeParse({ ...LAYOUT, header });
      expect(result.success && result.data.header).toMatchObject({ generated_by: "sv2 0.9.0" });
    });

    test("a record of an unknown kind survives whole", () => {
      const note = {
        kind: "annotation",
        element: "crisp-harbor-042",
        text: "reviewed",
        at: [10, 12],
      };
      const result = ViewLayoutSchema.safeParse({ ...LAYOUT, unknown: [note] });
      expect(result.success).toBe(true);
      expect(result.success && result.data.unknown[0]).toEqual(note);
    });

    test("known fields are still checked when unknown ones ride along", () => {
      // Loose means carrying what is unknown, not relaxing what is known.
      const node = { ...PINNED_NODE, extra: true, x: 320.5 };
      expect(ViewLayoutSchema.safeParse({ ...LAYOUT, nodes: [node] }).success).toBe(false);
    });
  });

  test.each([
    ["a schema this build does not read (R-7)", { ...LAYOUT, header: { ...HEADER, schema: 2 } }],
    [
      "a view that is not a petname",
      { ...LAYOUT, header: { ...HEADER, view: "views/amber-lattice-003" } },
    ],
    ["a zero grid", { ...LAYOUT, header: { ...HEADER, grid: 0 } }],
    [
      "the file's snake_case engine_version (§4.2)",
      {
        ...LAYOUT,
        header: (({ engineVersion: _e, ...rest }) => ({ ...rest, engine_version: "0.4.0" }))(
          HEADER,
        ),
      },
    ],
    ["a floating-point coordinate (R-1)", { ...LAYOUT, nodes: [{ ...PINNED_NODE, x: 320.5 }] }],
    ["a negative width", { ...LAYOUT, nodes: [{ ...PINNED_NODE, w: -1 }] }],
    [
      "a record with no src (R-4)",
      { ...LAYOUT, nodes: [(({ src: _s, ...rest }) => rest)(PINNED_NODE)] },
    ],
    ["a src outside R-4", { ...LAYOUT, nodes: [{ ...PINNED_NODE, src: "manual" }] }],
    [
      "an element keyed by a bare string",
      { ...LAYOUT, nodes: [{ ...PINNED_NODE, element: "crisp-harbor-042" }] },
    ],
    [
      "an element keyed by an unidentified span (ADR-0020)",
      {
        ...LAYOUT,
        nodes: [{ ...PINNED_NODE, element: { kind: "unidentified", span: { start: 0, end: 4 } } }],
      },
    ],
    [
      "a waypoint with three coordinates",
      { ...LAYOUT, edges: [{ ...EDGE, waypoints: [[1, 2, 3]] }] },
    ],
    ["a node record carrying a kind key", { ...LAYOUT, nodes: [{ ...PINNED_NODE, kind: "node" }] }],
    [
      // A valid edge in every field but one, so only the kind rule rejects it.
      "an edge-shaped record whose kind key says it is something else",
      { ...LAYOUT, edges: [{ ...EDGE, kind: "state" }] },
    ],
    [
      "a known kind in the unknown list",
      { ...LAYOUT, unknown: [{ kind: "node", ...PINNED_NODE }] },
    ],
    ["an unknown record with no kind", { ...LAYOUT, unknown: [{ element: "crisp-harbor-042" }] }],
    ["an extra key on the envelope, which is ours and not the sidecar's", { ...LAYOUT, style: [] }],
  ])("%s is not a layout", (_name, value) => {
    expect(ViewLayoutSchema.safeParse(value).success).toBe(false);
  });

  test("the schema this build reads is 1", () => {
    expect(LAYOUT_SCHEMA).toBe(1);
  });
});
