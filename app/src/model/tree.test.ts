// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { type Workspace, WorkspaceSchema } from "@/contract/file";
import { buildFileTree, type FileTreeNode } from "./tree";

const NONE = { error: 0, warning: 0, info: 0 };

/**
 * Parses a fixture through the application's own schema (§11), so the tree is
 * built from values that carry real brands rather than asserted ones.
 */
function workspace(name: string, files: ReadonlyArray<readonly [string, number?]>): Workspace {
  const result = WorkspaceSchema.safeParse({
    name,
    files: files.map(([path, errors = 0]) => ({
      path,
      language: path.endsWith(".kerml") ? "kerml" : "sysml",
      diagnostics: { ...NONE, error: errors },
    })),
  });
  if (!result.success) {
    throw new Error("fixture does not satisfy WorkspaceSchema");
  }
  return result.data;
}

/** Row names, one level. */
function names(nodes: readonly FileTreeNode[]): string[] {
  return nodes.map((node) => node.name);
}

/** The folder called `name` among `nodes`, or a failed test. */
function folder(
  nodes: readonly FileTreeNode[],
  name: string,
): Extract<FileTreeNode, { kind: "folder" }> {
  const found = nodes.find((node) => node.kind === "folder" && node.name === name);
  if (found === undefined || found.kind !== "folder") {
    throw new Error(`no folder ${name}`);
  }
  return found;
}

describe("buildFileTree", () => {
  // The mockup's Files tree (SCR-01), with its one error.
  const MOCKUP = workspace("ThermalControl", [
    ["model/ThermalControl.sysml", 1],
    ["model/Interfaces.sysml"],
    ["model/Requirements.sysml"],
    ["model/Behavior.sysml"],
    ["library/Units.kerml"],
  ]);

  test("the root row is the workspace", () => {
    expect(buildFileTree(MOCKUP).name).toBe("ThermalControl");
  });

  test("paths become folders, sorted, with files sorted inside them", () => {
    const tree = buildFileTree(MOCKUP);
    expect(names(tree.children)).toEqual(["library", "model"]);
    expect(names(folder(tree.children, "model").children)).toEqual([
      "Behavior.sysml",
      "Interfaces.sysml",
      "Requirements.sysml",
      "ThermalControl.sysml",
    ]);
  });

  test("an error in a file is counted on every folder above it (IX-10)", () => {
    const tree = buildFileTree(MOCKUP);
    expect(folder(tree.children, "model").diagnostics.error).toBe(1);
    expect(folder(tree.children, "library").diagnostics.error).toBe(0);
    expect(tree.diagnostics.error).toBe(1);
  });

  test("every severity is summed, not only errors", () => {
    const result = WorkspaceSchema.safeParse({
      name: "W",
      files: [
        { path: "a/x.sysml", language: "sysml", diagnostics: { error: 1, warning: 2, info: 3 } },
        { path: "a/y.sysml", language: "sysml", diagnostics: { error: 0, warning: 5, info: 1 } },
      ],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(buildFileTree(result.data).diagnostics).toEqual({ error: 1, warning: 7, info: 4 });
    }
  });

  test("folders come before files at the same level", () => {
    const tree = buildFileTree(workspace("W", [["Aardvark.sysml"], ["zoo/Z.sysml"]]));
    expect(names(tree.children)).toEqual(["zoo", "Aardvark.sysml"]);
  });

  test("numbers sort naturally", () => {
    const tree = buildFileTree(workspace("W", [["v10.sysml"], ["v2.sysml"], ["v1.sysml"]]));
    expect(names(tree.children)).toEqual(["v1.sysml", "v2.sysml", "v10.sysml"]);
  });

  test("case does not separate names, and case-only ties break the same way every run", () => {
    const tree = buildFileTree(
      workspace("W", [["units.kerml"], ["Beta.sysml"], ["Units.kerml"], ["alpha.sysml"]]),
    );
    // Code points put "U" before "u", whatever the machine's locale.
    expect(names(tree.children)).toEqual([
      "alpha.sysml",
      "Beta.sysml",
      "Units.kerml",
      "units.kerml",
    ]);
  });

  test("a folder's key is its path, so two folders with one name stay distinct", () => {
    const tree = buildFileTree(workspace("W", [["a/model/x.sysml"], ["b/model/y.sysml"]]));
    const a = folder(folder(tree.children, "a").children, "model");
    const b = folder(folder(tree.children, "b").children, "model");
    expect([a.key, b.key]).toEqual(["a/model", "b/model"]);
  });

  test("a file keeps the whole WorkspaceFile, so a click can open it by path", () => {
    const tree = buildFileTree(workspace("W", [["m/x.sysml", 2]]));
    const [row] = folder(tree.children, "m").children;
    expect(row?.kind === "file" ? String(row.file.path) : null).toBe("m/x.sysml");
    expect(row?.kind === "file" && row.file.diagnostics.error).toBe(2);
  });

  test("an empty workspace is a root with no children and no counts", () => {
    const tree = buildFileTree(workspace("Empty", []));
    expect(tree.children).toEqual([]);
    expect(tree.diagnostics).toEqual(NONE);
  });

  test("the input order does not matter", () => {
    const forward = buildFileTree(workspace("W", [["b/y.sysml"], ["a/x.sysml"], ["c.sysml"]]));
    const reverse = buildFileTree(workspace("W", [["c.sysml"], ["a/x.sysml"], ["b/y.sysml"]]));
    expect(forward).toEqual(reverse);
  });
});
