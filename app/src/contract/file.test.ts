// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import {
  DiagnosticCountsSchema,
  LANGUAGES,
  LanguageSchema,
  WorkspaceFileSchema,
  WorkspacePathSchema,
  WorkspaceSchema,
} from "./file";

// The workspace is the mockup's Files tree (SCR-01): ThermalControl, with
// model/ and library/ folders, and one error in ThermalControl.sysml.

const NONE = { error: 0, warning: 0, info: 0 } as const;

const THERMAL_CONTROL = {
  path: "model/ThermalControl.sysml",
  language: "sysml",
  diagnostics: { error: 1, warning: 0, info: 0 },
} as const;

const UNITS = { path: "library/Units.kerml", language: "kerml", diagnostics: NONE } as const;

const WORKSPACE = {
  name: "ThermalControl",
  files: [
    THERMAL_CONTROL,
    { path: "model/Interfaces.sysml", language: "sysml", diagnostics: NONE },
    UNITS,
  ],
} as const;

describe("WorkspacePathSchema", () => {
  test.each([
    ["a file in a folder", "model/ThermalControl.sysml"],
    ["a file at the root", "ThermalControl.sysml"],
    ["deep nesting", "a/b/c/d.kerml"],
    ["spaces and non-ASCII, which are legal file names", "modèle/Système thermique.sysml"],
    ["a dotfile, which is a name and not a . segment", ".hidden/x.sysml"],
    ["dots inside a name", "v1.2/x..y.sysml"],
  ])("%s is a workspace path", (_name, value) => {
    const result = WorkspacePathSchema.safeParse(value);
    expect(result.success).toBe(true);
    // Branding is a transform that loses nothing (§4.3 rule 4).
    expect(String(result.success && result.data)).toBe(value);
  });

  test.each([
    ["empty", ""],
    ["absolute", "/Users/someone/model/ThermalControl.sysml"],
    ["a Windows separator", "model\\ThermalControl.sysml"],
    ["a parent segment", "../outside.sysml"],
    ["a parent segment in the middle", "model/../ThermalControl.sysml"],
    ["a . segment", "./model/ThermalControl.sysml"],
    ["a doubled separator", "model//ThermalControl.sysml"],
    ["a trailing separator, which names a folder", "model/"],
    ["a NUL", "model/Thermal\0Control.sysml"],
  ])("%s is not a workspace path", (_name, value) => {
    expect(WorkspacePathSchema.safeParse(value).success).toBe(false);
  });

  test("a qualified name is not rejected by shape, which is why the brand exists", () => {
    // `ThermalControl` is a legal file name and a legal qualified name. The
    // schema cannot tell them apart; the type system, via the brand, does.
    expect(WorkspacePathSchema.safeParse("ThermalControl").success).toBe(true);
  });
});

describe("LanguageSchema", () => {
  test.each([...LANGUAGES])("%s is a language", (value) => {
    expect(LanguageSchema.safeParse(value).success).toBe(true);
  });

  test.each([
    ["an extension", ".sysml"],
    ["title case", "SysML"],
    ["a sidecar format", "jsonl"],
    ["empty", ""],
  ])("%s is not a language", (_name, value) => {
    expect(LanguageSchema.safeParse(value).success).toBe(false);
  });
});

describe("DiagnosticCountsSchema", () => {
  test("all three counts parse, including zeros", () => {
    expect(DiagnosticCountsSchema.safeParse(NONE).success).toBe(true);
  });

  test.each([
    ["a missing severity", { error: 1, warning: 0 }],
    ["a severity the core does not raise", { ...NONE, fatal: 1 }],
    ["a negative count", { ...NONE, error: -1 }],
    ["a fractional count", { ...NONE, error: 0.5 }],
    ["a count above u32 (§4.2)", { ...NONE, error: 2 ** 32 }],
  ])("%s is not a set of counts", (_name, value) => {
    expect(DiagnosticCountsSchema.safeParse(value).success).toBe(false);
  });
});

describe("WorkspaceFileSchema", () => {
  test("the mockup's ThermalControl.sysml parses, error count and all", () => {
    const result = WorkspaceFileSchema.safeParse(THERMAL_CONTROL);
    expect(result.success).toBe(true);
    expect(result.success && result.data.diagnostics.error).toBe(1);
  });

  test("language is taken at its word, not checked against the extension", () => {
    // Which grammar parsed the file is the core's call. If it ever parses a
    // file under a name that implies the other language, the badge follows it.
    expect(WorkspaceFileSchema.safeParse({ ...UNITS, language: "sysml" }).success).toBe(true);
  });

  test.each([
    ["a bad path", { ...THERMAL_CONTROL, path: "/abs/ThermalControl.sysml" }],
    ["no language", { path: THERMAL_CONTROL.path, diagnostics: NONE }],
    ["an extra key", { ...THERMAL_CONTROL, absolutePath: "/Users/someone/x.sysml" }],
  ])("%s is not a workspace file", (_name, value) => {
    expect(WorkspaceFileSchema.safeParse(value).success).toBe(false);
  });
});

describe("WorkspaceSchema", () => {
  test("the mockup's workspace parses", () => {
    expect(WorkspaceSchema.safeParse(WORKSPACE).success).toBe(true);
  });

  test("an empty workspace is a workspace", () => {
    expect(WorkspaceSchema.safeParse({ name: "Empty", files: [] }).success).toBe(true);
  });

  test("paths differing only by case are two files, as they are on a case-sensitive disk", () => {
    const files = [UNITS, { ...UNITS, path: "library/units.kerml" }];
    expect(WorkspaceSchema.safeParse({ name: "Cased", files }).success).toBe(true);
  });

  test.each([
    ["two files at one path", { ...WORKSPACE, files: [UNITS, UNITS] }],
    ["no name", { files: [] }],
    ["an empty name", { name: "", files: [] }],
    [
      "a nested folder node, which the flat wire has no place for",
      {
        ...WORKSPACE,
        folders: [{ name: "model" }],
      },
    ],
  ])("%s is not a workspace", (_name, value) => {
    expect(WorkspaceSchema.safeParse(value).success).toBe(false);
  });
});
