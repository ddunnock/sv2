// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { type ElementHandle, ElementHandleSchema } from "@/contract/element-id";
import { assertNever } from "./assert-never";
import { logLabel, sameHandle } from "./element-handle";

/** Parses a fixture through the application's own schema (§11). */
function handle(raw: unknown): ElementHandle {
  const result = ElementHandleSchema.safeParse(raw);
  if (!result.success) {
    throw new Error("fixture does not satisfy ElementHandleSchema");
  }
  return result.data;
}

const PETNAME = handle({ kind: "petname", id: "maple-sunrise-314" });
const MEMBERSHIP = handle({ kind: "membership", of: "maple-sunrise-314" });
const DERIVED = handle({ kind: "derived", hash: "9f2c1ab04e" });
const LIBRARY = handle({ kind: "library", uuid: "e5b6a8f0-3d2c-5a1b-9c4d-0123456789ab" });
const UNIDENTIFIED = handle({ kind: "unidentified", span: { start: 26, end: 40 } });

describe("logLabel", () => {
  test.each([
    ["a petname is itself (§9.3)", PETNAME, "maple-sunrise-314"],
    ["a membership is ADR-0016's <owned-ID>/m", MEMBERSHIP, "maple-sunrise-314/m"],
    ["a derived handle says its namespace", DERIVED, "derived:9f2c1ab04e"],
    [
      "a library element says its namespace",
      LIBRARY,
      "library:e5b6a8f0-3d2c-5a1b-9c4d-0123456789ab",
    ],
    ["an unidentified element is its span, two counts", UNIDENTIFIED, "unidentified@26-40"],
  ])("%s", (_name, value, label) => {
    expect(logLabel(value)).toBe(label);
  });

  test("no two arms can produce the same label", () => {
    const labels = [PETNAME, MEMBERSHIP, DERIVED, LIBRARY, UNIDENTIFIED].map(logLabel);
    expect(new Set(labels).size).toBe(labels.length);
  });
});

describe("sameHandle", () => {
  test("two separately parsed copies of one handle are the same", () => {
    expect(sameHandle(PETNAME, handle({ kind: "petname", id: "maple-sunrise-314" }))).toBe(true);
    expect(
      sameHandle(UNIDENTIFIED, handle({ kind: "unidentified", span: { start: 26, end: 40 } })),
    ).toBe(true);
  });

  test("an element and its owning membership are different things", () => {
    // Same petname inside, different element: ADR-0016 rows 1 and 2.
    expect(sameHandle(PETNAME, MEMBERSHIP)).toBe(false);
    expect(sameHandle(MEMBERSHIP, PETNAME)).toBe(false);
  });

  test.each([
    ["petnames", PETNAME, handle({ kind: "petname", id: "cedar-harbor-208" })],
    ["derived hashes", DERIVED, handle({ kind: "derived", hash: "0000aaaa" })],
    [
      "library elements",
      LIBRARY,
      handle({ kind: "library", uuid: "0b6e1a2c-9d3f-5e4a-8b7c-112233445566" }),
    ],
    ["spans", UNIDENTIFIED, handle({ kind: "unidentified", span: { start: 26, end: 41 } })],
  ])("different %s are different", (_name, a, b) => {
    expect(sameHandle(a, b)).toBe(false);
  });
});

describe("assertNever", () => {
  /** Reaches assertNever with a value the compiler believes impossible, as a bypassed schema would. */
  function reach(value: unknown): unknown {
    return Reflect.apply(assertNever, undefined, [value]);
  }

  test("it throws, naming the discriminant", () => {
    expect(() => reach({ kind: "partial" })).toThrow("unhandled variant: kind=partial");
    expect(() => reach({ status: "stale" })).toThrow("unhandled variant: status=stale");
  });

  test("it never puts the value's other fields in the message (§9.3)", () => {
    // `written` is model text: what the author typed.
    const leaked = { kind: "partial", written: "Vehicles::Vehicle::engine" };
    expect(() => reach(leaked)).toThrow(/^unhandled variant: kind=partial$/);
  });

  test("a value with no discriminant says so rather than printing itself", () => {
    expect(() => reach("Vehicles::Vehicle::engine")).toThrow(
      /^unhandled variant: no discriminant$/,
    );
  });
});
