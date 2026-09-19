// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { type TreeNode, treeMove, visibleRows } from "./tree-rows";

// The mockup's Files tree, as nodes.
const NODES: readonly TreeNode[] = [
  {
    id: "model",
    label: "model",
    children: [
      { id: "tc", label: "ThermalControl.sysml", children: null },
      { id: "if", label: "Interfaces.sysml", children: null },
    ],
  },
  {
    id: "library",
    label: "library",
    children: [{ id: "units", label: "Units.kerml", children: null }],
  },
  { id: "empty", label: "verification", children: [] },
];

const ids = (expanded: readonly string[]): string[] =>
  visibleRows(NODES, new Set(expanded)).map((row) => row.node.id);

describe("visibleRows", () => {
  test("a collapsed tree shows only its roots", () => {
    expect(ids([])).toEqual(["model", "library", "empty"]);
  });

  test("an expanded parent shows its children, in order, directly after it", () => {
    expect(ids(["model"])).toEqual(["model", "tc", "if", "library", "empty"]);
  });

  test("each row states its level, position, set size and parent", () => {
    const [, second] = visibleRows(NODES, new Set(["model"]));
    expect(second).toMatchObject({ level: 2, posinset: 1, setsize: 2, parent: "model" });
  });

  test("an expanded empty parent adds nothing", () => {
    expect(ids(["empty"])).toEqual(["model", "library", "empty"]);
  });
});

describe("treeMove", () => {
  const at = (expanded: readonly string[], id: string) => {
    const rows = visibleRows(NODES, new Set(expanded));
    return {
      rows,
      current: rows.findIndex((row) => row.node.id === id),
      expanded: new Set(expanded),
    };
  };

  test("down and up step without wrapping (the APG tree does not wrap)", () => {
    expect(treeMove("ArrowDown", at([], "model"))).toEqual({ kind: "focus", index: 1 });
    expect(treeMove("ArrowDown", at([], "empty"))).toEqual({ kind: "none" });
    expect(treeMove("ArrowUp", at([], "model"))).toEqual({ kind: "none" });
  });

  test("Home and End reach the first and last visible rows", () => {
    expect(treeMove("End", at(["model"], "model"))).toEqual({ kind: "focus", index: 4 });
    expect(treeMove("Home", at(["model"], "empty"))).toEqual({ kind: "focus", index: 0 });
  });

  test("right on a closed parent opens it", () => {
    expect(treeMove("ArrowRight", at([], "model"))).toEqual({ kind: "toggle", id: "model" });
  });

  test("right on an open parent moves to its first child", () => {
    expect(treeMove("ArrowRight", at(["model"], "model"))).toEqual({ kind: "focus", index: 1 });
  });

  test("right on an open empty parent, or on a leaf, does nothing", () => {
    expect(treeMove("ArrowRight", at(["empty"], "empty"))).toEqual({ kind: "none" });
    expect(treeMove("ArrowRight", at(["model"], "tc"))).toEqual({ kind: "none" });
  });

  test("left on an open parent closes it", () => {
    expect(treeMove("ArrowLeft", at(["model"], "model"))).toEqual({ kind: "toggle", id: "model" });
  });

  test("left on a child moves to its parent", () => {
    expect(treeMove("ArrowLeft", at(["model"], "if"))).toEqual({ kind: "focus", index: 0 });
  });

  test("left on a closed root does nothing", () => {
    expect(treeMove("ArrowLeft", at([], "library"))).toEqual({ kind: "none" });
  });

  test("any other key is not the tree's", () => {
    expect(treeMove("x", at([], "model"))).toEqual({ kind: "none" });
  });
});
