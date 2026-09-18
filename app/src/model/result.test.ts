// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Tests for `result.ts`, beside the module they test (STD-004-TS §2, rule 2).
 *
 * `bun:test` is available here and NOT in the application modules: Bun runs the
 * toolchain, the application runs in the Tauri webview, and tsconfig.test.json
 * is what separates the two (§2, rule 5).
 */

import { describe, expect, test } from "bun:test";
import { err, isOk, ok, type Result } from "./result";

describe("Result", () => {
  test("ok carries its value and narrows", () => {
    const result: Result<number, string> = ok(1);
    expect(isOk(result)).toBe(true);
    if (isOk(result)) {
      // The narrowing is the point: `value` is only reachable on the ok arm.
      expect(result.value).toBe(1);
    }
  });

  test("err carries its error and does not narrow to ok", () => {
    const result: Result<number, string> = err("no");
    expect(isOk(result)).toBe(false);
    expect(result.ok ? null : result.error).toBe("no");
  });
});
