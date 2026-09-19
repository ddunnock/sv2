// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { type Workspace, WorkspaceSchema } from "@/contract/file";
import { buildFileTree } from "@/model/tree";

import { fileNodes, folderIds, plural, ROOT_ID } from "./file-nodes";

function workspace(files: ReadonlyArray<readonly [string, number, number]>): Workspace {
  const result = WorkspaceSchema.safeParse({
    name: "ThermalControl",
    files: files.map(([path, error, warning]) => ({
      path,
      language: "sysml",
      diagnostics: { error, warning, info: 0 },
    })),
  });
  if (!result.success) {
    throw new Error("fixture does not satisfy WorkspaceSchema");
  }
  return result.data;
}

const nodes = (files: ReadonlyArray<readonly [string, number, number]>) =>
  fileNodes(buildFileTree(workspace(files)));

describe("fileNodes", () => {
  test("the workspace is the one root, named for it", () => {
    const [root] = nodes([["model/A.sysml", 0, 0]]);
    expect(root?.id).toBe(ROOT_ID);
    expect(root?.label).toBe("ThermalControl");
  });

  test("ids are namespaced, so a folder and a file cannot share one", () => {
    const [root] = nodes([["model/A.sysml", 0, 0]]);
    const model = root?.children?.[0];
    expect(model?.id).toBe("folder:model");
    expect(model?.children?.[0]?.id).toBe("file:model/A.sysml");
  });

  test("errors are shown in the error tone", () => {
    const [root] = nodes([["A.sysml", 2, 5]]);
    expect(root?.children?.[0]).toMatchObject({ description: "2 errors", tone: "error" });
  });

  test("warnings show only when there are no errors, muted", () => {
    const [root] = nodes([["A.sysml", 0, 1]]);
    expect(root?.children?.[0]).toMatchObject({ description: "1 warning", tone: "muted" });
  });

  test("a clean file says nothing", () => {
    const [root] = nodes([["A.sysml", 0, 0]]);
    expect(root?.children?.[0]?.description).toBeUndefined();
  });

  test("folderIds lists every parent, root included, and no file", () => {
    expect(
      folderIds(
        nodes([
          ["a/b/C.sysml", 0, 0],
          ["D.sysml", 0, 0],
        ]),
      ),
    ).toEqual([ROOT_ID, "folder:a", "folder:a/b"]);
  });
});

describe("plural", () => {
  test.each([
    [0, "0 errors"],
    [1, "1 error"],
    [2, "2 errors"],
  ])("%d", (count, text) => {
    expect(plural(count, "error")).toBe(text);
  });
});
