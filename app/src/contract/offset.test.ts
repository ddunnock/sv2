// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { TextSpanSchema, Utf16OffsetSchema } from "./offset";

describe("Utf16OffsetSchema", () => {
  test("a whole non-negative position parses to itself", () => {
    const result = Utf16OffsetSchema.safeParse(42);
    expect(result.success).toBe(true);
    if (!result.success) {
      return;
    }
    // The brand is type-only: the transform must not change the value, or every
    // offset the webview sends back to Rust would be wrong.
    //
    // `Number(...)` rather than comparing the branded value directly, because
    // the compiler refuses `expect(result.data).toBe(42)` — a plain 42 is not a
    // Utf16Offset. That refusal is the brand working, so it is read around here
    // rather than asserted away.
    expect(Number(result.data)).toBe(42);
  });

  test("the start of a document is a position", () => {
    expect(Utf16OffsetSchema.safeParse(0).success).toBe(true);
  });

  test.each([
    ["negative", -1],
    ["fractional", 1.5],
    ["not a number", "7"],
    ["null", null],
    ["NaN", Number.NaN],
    ["infinite", Number.POSITIVE_INFINITY],
  ])("a %s value is not a position", (_name, value) => {
    expect(Utf16OffsetSchema.safeParse(value).success).toBe(false);
  });
});

describe("TextSpanSchema", () => {
  test("a span parses both ends", () => {
    const result = TextSpanSchema.safeParse({ start: 3, end: 9 });
    expect(result.success).toBe(true);
    if (!result.success) {
      return;
    }
    expect(Number(result.data.start)).toBe(3);
    expect(Number(result.data.end)).toBe(9);
  });

  test("an empty span is legal, because a diagnostic points between characters", () => {
    // sv2-syntax raises these: `Diagnostic::range` may be empty, and an
    // "expected X here" has nowhere else to attach.
    expect(TextSpanSchema.safeParse({ start: 4, end: 4 }).success).toBe(true);
  });

  test("an unknown key is rejected, because the object is strict (§4.3 rule 2)", () => {
    expect(TextSpanSchema.safeParse({ start: 0, end: 1, file: "a.sysml" }).success).toBe(false);
  });

  test.each([
    ["a missing end", { start: 0 }],
    ["a negative end", { start: 0, end: -1 }],
    ["a fractional start", { start: 0.5, end: 1 }],
  ])("%s is not a span", (_name, value) => {
    expect(TextSpanSchema.safeParse(value).success).toBe(false);
  });

  test("an inverted span still parses, because ordering is not refined here", () => {
    // Deliberate, and §4.7 is the reason: a refinement would run on every span
    // on a per-keystroke path. Recorded as a test so that if someone adds the
    // refinement later, they do it knowingly and this test moves with them.
    expect(TextSpanSchema.safeParse({ start: 9, end: 3 }).success).toBe(true);
  });
});
