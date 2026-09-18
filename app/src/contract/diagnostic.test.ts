// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import {
  DiagnosticCodeSchema,
  DiagnosticSchema,
  PARSE_CODES,
  SEVERITIES,
  SeveritySchema,
} from "./diagnostic";

const A_DIAGNOSTIC = {
  code: PARSE_CODES.unexpected,
  severity: "error",
  span: { start: 26, end: 27 },
  message: "unexpected `x`",
};

describe("SeveritySchema", () => {
  // Spread because SEVERITIES is `as const` and `test.each` wants a mutable
  // array. The readonly is the point of §4.6 rule 3, so it is copied here
  // rather than relaxed at the source.
  test.each([...SEVERITIES])("%s is a severity", (value) => {
    expect(SeveritySchema.safeParse(value).success).toBe(true);
  });

  test.each([
    ["title case", "Error"],
    ["a level sv2-syntax does not raise", "fatal"],
    ["a number", 1],
    ["empty", ""],
  ])("%s is not a severity", (_name, value) => {
    expect(SeveritySchema.safeParse(value).success).toBe(false);
  });

  test("the three mirror sv2_syntax::Severity::as_str", () => {
    expect([...SEVERITIES]).toEqual(["error", "warning", "info"]);
  });
});

describe("DiagnosticCodeSchema", () => {
  test.each(Object.values(PARSE_CODES))("%s is a code", (code) => {
    expect(DiagnosticCodeSchema.safeParse(code).success).toBe(true);
  });

  test.each([
    ["a resolver code that does not exist yet", "RES-UNRESOLVED-NAME"],
    ["an identity code that does not exist yet", "ID-DUPLICATE"],
    ["a long one", "HIR-IMPLIED-SPECIALIZATION-CYCLE"],
  ])("%s is admitted, because the enum is open (ADR-0002)", (_name, code) => {
    // The point of the shape check. A webview that rejected an unfamiliar code
    // would drop the report that something is wrong because the reason is new,
    // which is filtering on diagnostic state rather than decorating by it.
    expect(DiagnosticCodeSchema.safeParse(code).success).toBe(true);
  });

  test.each([
    ["no namespace", "EXPECTED"],
    ["lowercase", "parse-expected"],
    ["mixed case", "Parse-Expected"],
    ["a trailing hyphen", "PARSE-"],
    ["a leading hyphen", "-EXPECTED"],
    ["a double hyphen", "PARSE--EXPECTED"],
    ["digits", "PARSE-E2"],
    ["a space", "PARSE EXPECTED"],
    ["empty", ""],
    // sv2-cli's ErrorCode is a process taxonomy, one code per exit status, and
    // its own module says it must never be merged with this one. The spelling
    // keeps them apart on the wire as well as in Rust.
    ["an sv2-cli ErrorCode", "NOT_IMPLEMENTED"],
  ])("%s is not a code", (_name, value) => {
    expect(DiagnosticCodeSchema.safeParse(value).success).toBe(false);
  });
});

describe("DiagnosticSchema", () => {
  test("a parse diagnostic parses", () => {
    const result = DiagnosticSchema.safeParse(A_DIAGNOSTIC);
    expect(result.success).toBe(true);
  });

  test("an empty span is fine, because that is where 'expected X here' attaches", () => {
    expect(
      DiagnosticSchema.safeParse({ ...A_DIAGNOSTIC, span: { start: 12, end: 12 } }).success,
    ).toBe(true);
  });

  test("a message may be empty, because the code still says what happened", () => {
    expect(DiagnosticSchema.safeParse({ ...A_DIAGNOSTIC, message: "" }).success).toBe(true);
  });

  test("severity is read, not re-derived, so a warning is taken at its word", () => {
    // Every code sv2-syntax raises today is an error. If the core starts
    // raising this one as a warning, the webview follows without a change here
    // — which is the whole reason severity is carried.
    const result = DiagnosticSchema.safeParse({ ...A_DIAGNOSTIC, severity: "warning" });
    expect(result.success).toBe(true);
    expect(result.success && result.data.severity).toBe("warning");
  });

  test.each([
    ["a missing span", { code: PARSE_CODES.expected, severity: "error", message: "m" }],
    [
      "a missing severity",
      { code: PARSE_CODES.expected, span: { start: 0, end: 1 }, message: "m" },
    ],
    ["a bad code", { ...A_DIAGNOSTIC, code: "oops" }],
    ["a negative offset", { ...A_DIAGNOSTIC, span: { start: -1, end: 0 } }],
    ["an extra key", { ...A_DIAGNOSTIC, file: "model/ThermalControl.sysml" }],
  ])("%s is not a diagnostic", (_name, value) => {
    expect(DiagnosticSchema.safeParse(value).success).toBe(false);
  });
});
