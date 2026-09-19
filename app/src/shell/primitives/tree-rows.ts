// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The visible rows of a tree, and where each tree-view key moves focus
 * (WAI-ARIA APG, "Tree View"). Pure, so the keyboard is tested without a DOM.
 *
 * FLAT ROWS, STRUCTURE IN ATTRIBUTES. The APG allows a tree whose DOM does not
 * nest, provided each item states its `aria-level`, `aria-setsize` and
 * `aria-posinset`. Rendering a flat list is what makes a row's position an
 * index, and the keyboard below a matter of arithmetic on that index.
 */

/** One node. `children` is null for a leaf and an array — possibly empty — for a parent. */
export type TreeNode = Readonly<{
  id: string;
  label: string;
  /** Shown after the label, and read after it by assistive technology: "1 error". */
  description?: string;
  /** How the description is drawn. An error count is `error`; anything else is `muted`. */
  tone?: "error" | "muted";
  children: readonly TreeNode[] | null;
}>;

/** A node as it is drawn: where it sits, and who its parent is. */
export type TreeRow = Readonly<{
  node: TreeNode;
  level: number;
  posinset: number;
  setsize: number;
  parent: string | null;
}>;

/** The rows a person can see: every root, and the children of every expanded parent. */
export function visibleRows(
  nodes: readonly TreeNode[],
  expanded: ReadonlySet<string>,
): readonly TreeRow[] {
  const rows: TreeRow[] = [];
  const walk = (level: readonly TreeNode[], depth: number, parent: string | null): void => {
    level.forEach((node, index) => {
      rows.push({ node, level: depth, posinset: index + 1, setsize: level.length, parent });
      if (node.children !== null && expanded.has(node.id)) {
        walk(node.children, depth + 1, node.id);
      }
    });
  };
  walk(nodes, 1, null);
  return rows;
}

/** What a key does: move focus to a row, expand or collapse one, or nothing. */
export type TreeMove =
  | Readonly<{ kind: "focus"; index: number }>
  | Readonly<{ kind: "toggle"; id: string }>
  | Readonly<{ kind: "none" }>;

const NONE: TreeMove = { kind: "none" };

/**
 * What `key` does from the row at `current`.
 *
 * Up and Down step without wrapping — the APG tree does not wrap, unlike a
 * toolbar. Right opens a closed parent, and on an open one moves to its first
 * child. Left closes an open parent, and otherwise moves to the parent. Home
 * and End reach the first and last visible rows.
 */
export function treeMove(
  key: string,
  state: Readonly<{ rows: readonly TreeRow[]; current: number; expanded: ReadonlySet<string> }>,
): TreeMove {
  const { rows, current } = state;
  const row = rows[current];
  if (row === undefined) {
    return NONE;
  }
  switch (key) {
    case "ArrowDown":
      return current < rows.length - 1 ? { kind: "focus", index: current + 1 } : NONE;
    case "ArrowUp":
      return current > 0 ? { kind: "focus", index: current - 1 } : NONE;
    case "Home":
      return { kind: "focus", index: 0 };
    case "End":
      return { kind: "focus", index: rows.length - 1 };
    case "ArrowRight":
      return rightFrom(row, state);
    case "ArrowLeft":
      return leftFrom(row, state);
    default:
      return NONE;
  }
}

/** Right: open a closed parent, or step into an open one. */
function rightFrom(
  row: TreeRow,
  state: Readonly<{ current: number; expanded: ReadonlySet<string> }>,
): TreeMove {
  if (row.node.children === null) {
    return NONE;
  }
  if (!state.expanded.has(row.node.id)) {
    return { kind: "toggle", id: row.node.id };
  }
  return row.node.children.length > 0 ? { kind: "focus", index: state.current + 1 } : NONE;
}

/** Left: close an open parent, or step out to this row's parent. */
function leftFrom(
  row: TreeRow,
  state: Readonly<{ rows: readonly TreeRow[]; expanded: ReadonlySet<string> }>,
): TreeMove {
  if (row.node.children !== null && state.expanded.has(row.node.id)) {
    return { kind: "toggle", id: row.node.id };
  }
  const parent = state.rows.findIndex((candidate) => candidate.node.id === row.parent);
  return parent === -1 ? NONE : { kind: "focus", index: parent };
}
