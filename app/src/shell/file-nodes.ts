// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The Files tree as the `Tree` primitive draws it (IX-01, IX-10).
 *
 * `model/tree.ts` decides the structure and the order; this decides only what
 * each row says. Pure, so it is tested without rendering.
 *
 * IDS ARE NAMESPACED. A folder is `folder:<path>` and a file `file:<path>`, so
 * a folder and a file can never share an id even where a path could be read
 * as either, and a click can be routed back to the right kind of thing.
 *
 * THE COUNT THAT MATTERS. A row shows its errors if it has any, in the error
 * tone; otherwise its warnings, muted; otherwise nothing. The same words are
 * its accessible description, so a screen reader hears "model, 1 error".
 */

import type { DiagnosticCounts } from "@/contract/file";
import type { FileTree, FileTreeNode } from "@/model/tree";

import type { TreeNode } from "./primitives/tree-rows";

/** The id of the root row, which is the workspace itself. */
export const ROOT_ID = "workspace";

/** The Files tree's rows: the workspace as the one root, everything under it. */
export function fileNodes(tree: FileTree): readonly TreeNode[] {
  return [
    {
      id: ROOT_ID,
      label: tree.name,
      ...describe(tree.diagnostics),
      children: tree.children.map(toNode),
    },
  ];
}

/** Every folder id in the tree, including the root: all expanded is the default. */
export function folderIds(nodes: readonly TreeNode[]): readonly string[] {
  return nodes.flatMap((node) =>
    node.children === null ? [] : [node.id, ...folderIds(node.children)],
  );
}

function toNode(node: FileTreeNode): TreeNode {
  if (node.kind === "file") {
    return {
      id: `file:${node.file.path}`,
      label: node.name,
      ...describe(node.file.diagnostics),
      children: null,
    };
  }
  return {
    id: `folder:${node.key}`,
    label: node.name,
    ...describe(node.diagnostics),
    children: node.children.map(toNode),
  };
}

/** The description and tone for a set of counts; empty when there is nothing to say. */
function describe(counts: DiagnosticCounts): Pick<TreeNode, "description" | "tone"> {
  if (counts.error > 0) {
    return { description: plural(counts.error, "error"), tone: "error" };
  }
  if (counts.warning > 0) {
    return { description: plural(counts.warning, "warning"), tone: "muted" };
  }
  return {};
}

/** `1 error`, `2 errors`. */
export function plural(count: number, noun: string): string {
  return `${count} ${noun}${count === 1 ? "" : "s"}`;
}
