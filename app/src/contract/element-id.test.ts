// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { ElementHandleSchema, ElementIdSchema, ViewIdSchema } from "./element-id";

/** ADR-0016's own example, so the happy case is the ADR's and not one invented here. */
const PETNAME = "maple-sunrise-314";

describe("ElementIdSchema", () => {
  test("the ADR-0016 example parses unchanged", () => {
    const result = ElementIdSchema.safeParse(PETNAME);
    expect(result.success).toBe(true);
    expect(String(result.success && result.data)).toBe(PETNAME);
  });

  test.each([
    // The substitution §4.4 names first, and the one a resolver would make.
    ["a qualified name", "Vehicles::Vehicle::engine"],
    ["a bare name", "engine"],
    ["uppercase", "Maple-Sunrise-314"],
    ["two digits", "maple-sunrise-31"],
    ["four digits", "maple-sunrise-3141"],
    ["no digits", "maple-sunrise"],
    ["one word", "maple-314"],
    ["three words", "maple-sunrise-dawn-314"],
    ["digits first", "314-maple-sunrise"],
    ["an underscore", "maple_sunrise_314"],
    ["empty", ""],
    // Anchoring. Without ^ and $ each of these matches somewhere inside.
    ["a trailing character", "maple-sunrise-314x"],
    // NOT "xmaple-sunrise-314": a leading letter only makes a longer first
    // word, and that really is a petname. Anchoring is tested with characters
    // the grammar cannot absorb.
    ["a leading digit", "1maple-sunrise-314"],
    ["a leading space", " maple-sunrise-314"],
    ["surrounding text", "see maple-sunrise-314 here"],
    ["a newline after", "maple-sunrise-314\n"],
  ])("%s is not an identity", (_name, value) => {
    expect(ElementIdSchema.safeParse(value).success).toBe(false);
  });
});

describe("ViewIdSchema", () => {
  test("shares the petname grammar", () => {
    expect(ViewIdSchema.safeParse(PETNAME).success).toBe(true);
  });

  test("and rejects the same things", () => {
    expect(ViewIdSchema.safeParse("Views::PowerTree").success).toBe(false);
  });
});

describe("ElementHandleSchema", () => {
  test.each([
    ["a stored petname", { kind: "petname", id: PETNAME }],
    ["an owning membership", { kind: "membership", of: PETNAME }],
    ["a derived relationship", { kind: "derived", hash: "sha256:9f2c1ab4" }],
    ["a library element", { kind: "library", uuid: "0b2b2f6e-6b7a-4f4e-9c3a-2d1e8f0a7b55" }],
    ["an element still loading", { kind: "unidentified", span: { start: 0, end: 12 } }],
  ])("%s is a handle", (_name, value) => {
    expect(ElementHandleSchema.safeParse(value).success).toBe(true);
  });

  test("a derived identity shaped like a petname is rejected", () => {
    // ADR-0016 puts derived relationship identities in a namespace separate
    // from petnames. This is the checkable half of that: if a hash could be
    // mistaken for a stored identity, the two namespaces have collided and a
    // consumer cannot tell which table to look in.
    const result = ElementHandleSchema.safeParse({ kind: "derived", hash: PETNAME });
    expect(result.success).toBe(false);
  });

  test.each([
    ["an unknown kind", { kind: "guid", id: PETNAME }],
    ["a membership of a qualified name", { kind: "membership", of: "Vehicles::engine" }],
    ["a library element with a non-uuid", { kind: "library", uuid: PETNAME }],
    ["a petname arm carrying an extra key", { kind: "petname", id: PETNAME, file: "a.sysml" }],
    ["an empty derived hash", { kind: "derived", hash: "" }],
    ["no kind at all", { id: PETNAME }],
  ])("%s is not a handle", (_name, value) => {
    expect(ElementHandleSchema.safeParse(value).success).toBe(false);
  });

  test("the membership arm keeps the owned identity, not a derived string", () => {
    // ADR-0016 derives a membership as `<owned-ID>/m`. The handle carries the
    // owned identity so a consumer that wants the element does not parse a
    // suffix back off, and so `<owned-ID>/m` cannot arrive here as an identity.
    const result = ElementHandleSchema.safeParse({ kind: "membership", of: `${PETNAME}/m` });
    expect(result.success).toBe(false);
  });
});
