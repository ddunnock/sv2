// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { type WorkspacePath, WorkspacePathSchema } from "@/contract/file";

import { type EditorPlace, editorPlaceReducer, NO_EDITOR } from "./editor-place";

function path(text: string): WorkspacePath {
  const parsed = WorkspacePathSchema.safeParse(text);
  if (!parsed.success) {
    throw new Error(`test path ${text} does not satisfy WorkspacePathSchema`);
  }
  return parsed.data;
}

const A = path("model/A.sysml");
const B = path("model/B.sysml");
const SNAPSHOT = { text: "part def A;\n", state: { doc: "part def A;\n" } };

const docked = (at: WorkspacePath): EditorPlace => ({ kind: "docked", path: at, seed: null });
const undocked: EditorPlace = { kind: "undocked", path: A };

describe("opening a file", () => {
  test("from nothing, docks it, read from disk", () => {
    expect(editorPlaceReducer(NO_EDITOR, { kind: "open", path: A })).toEqual(docked(A));
  });

  test("another file replaces the docked one", () => {
    expect(editorPlaceReducer(docked(A), { kind: "open", path: B })).toEqual(docked(B));
  });

  test("the same file again keeps the editor it has, and its edits", () => {
    const withEdits: EditorPlace = { kind: "docked", path: A, seed: SNAPSHOT };
    expect(editorPlaceReducer(withEdits, { kind: "open", path: A })).toBe(withEdits);
  });

  test("while the editor is in its own window, opens nothing: the returning file needs its place", () => {
    expect(editorPlaceReducer(undocked, { kind: "open", path: B })).toBe(undocked);
  });
});

describe("closing", () => {
  test("closes the docked editor", () => {
    expect(editorPlaceReducer(docked(A), { kind: "close" })).toEqual(NO_EDITOR);
  });

  test("does not close an editor in its own window: its edits are there, not here", () => {
    expect(editorPlaceReducer(undocked, { kind: "close" })).toBe(undocked);
  });
});

describe("moving between windows", () => {
  test("undocking keeps only the path: the document went with the file", () => {
    expect(editorPlaceReducer(docked(A), { kind: "undocked" })).toEqual(undocked);
  });

  test("undocking with nothing docked changes nothing", () => {
    expect(editorPlaceReducer(NO_EDITOR, { kind: "undocked" })).toBe(NO_EDITOR);
  });

  test("docking back brings the snapshot, so it is not read from disk again", () => {
    expect(editorPlaceReducer(undocked, { kind: "docked", path: A, snapshot: SNAPSHOT })).toEqual({
      kind: "docked",
      path: A,
      seed: SNAPSHOT,
    });
  });
});
