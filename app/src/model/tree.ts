// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The Files tree (IX-01), built from the flat file list the core sends.
 *
 * `contract/file.ts` keeps the wire flat so a folder is stated once, as a path
 * prefix; this module is the one place that turns prefixes into folders. It is
 * pure — a workspace in, a tree out — so it is tested without a DOM.
 *
 * FOLDERS CARRY THEIR CONTENTS' COUNTS. IX-10 wants diagnostics wherever an
 * element appears, and a collapsed folder is where a file's error is hidden.
 * Summing on the way up means the shell never walks the tree to find them.
 *
 * ORDER: folders before files, then by name, compared naturally and without
 * regard to case, so `v2` sorts before `v10` and `units` beside `Units`. Names
 * equal under that comparison fall back to code-point order, so the order never
 * depends on the machine's locale and two runs always agree.
 */

import type { DiagnosticCounts, Workspace, WorkspaceFile } from "@/contract/file";

/** A file row: its own name, and the file it stands for. */
export type FileTreeFile = Readonly<{ kind: "file"; name: string; file: WorkspaceFile }>;

/**
 * A folder row. `key` is its path within the workspace, which is unique where
 * `name` is not — two folders can both be called `model`.
 */
export type FileTreeFolder = Readonly<{
  kind: "folder";
  name: string;
  key: string;
  diagnostics: DiagnosticCounts;
  children: readonly FileTreeNode[];
}>;

/** One row of the Files tree. */
export type FileTreeNode = FileTreeFolder | FileTreeFile;

/** The whole tree: the workspace's root row and everything under it. */
export type FileTree = Readonly<{
  name: string;
  diagnostics: DiagnosticCounts;
  children: readonly FileTreeNode[];
}>;

/** Accumulates one folder while the tree is built; owned by `buildFileTree` alone (§4.6). */
type FolderBuilder = {
  readonly folders: Map<string, FolderBuilder>;
  readonly files: FileTreeFile[];
};

const NONE: DiagnosticCounts = { error: 0, warning: 0, info: 0 };

/** Builds the Files tree for `workspace`. */
export function buildFileTree(workspace: Workspace): FileTree {
  const root: FolderBuilder = { folders: new Map(), files: [] };
  for (const file of workspace.files) {
    insert(root, file);
  }
  // Built here, not at module scope: constructing one is a call, and §3.3
  // allows no work at import time.
  const compare = nameOrder(new Intl.Collator("en", { numeric: true, sensitivity: "base" }));
  const children = freeze(root, "", compare);
  return { name: workspace.name, diagnostics: sumOf(children), children };
}

/** Files `file` under the folders its path names, creating them as needed. */
function insert(root: FolderBuilder, file: WorkspaceFile): void {
  const segments = file.path.split("/");
  const name = segments.pop() ?? file.path;
  let folder = root;
  for (const segment of segments) {
    let next = folder.folders.get(segment);
    if (next === undefined) {
      next = { folders: new Map(), files: [] };
      folder.folders.set(segment, next);
    }
    folder = next;
  }
  folder.files.push({ kind: "file", name, file });
}

/** Turns a builder into sorted, readonly rows, summing counts on the way up. */
function freeze(
  builder: FolderBuilder,
  prefix: string,
  compare: (a: FileTreeNode, b: FileTreeNode) => number,
): readonly FileTreeNode[] {
  const folders: FileTreeFolder[] = [...builder.folders].map(([name, child]) => {
    const key = prefix === "" ? name : `${prefix}/${name}`;
    const children = freeze(child, key, compare);
    return { kind: "folder", name, key, diagnostics: sumOf(children), children };
  });
  return [...folders, ...builder.files].sort(compare);
}

/** Folders first, then natural, case-blind name order, then code points to break ties. */
function nameOrder(collator: Intl.Collator): (a: FileTreeNode, b: FileTreeNode) => number {
  return (a, b) => {
    if (a.kind !== b.kind) {
      return a.kind === "folder" ? -1 : 1;
    }
    return collator.compare(a.name, b.name) || codePointOrder(a.name, b.name);
  };
}

/** Plain code-point comparison, which no locale can change. */
function codePointOrder(a: string, b: string): number {
  if (a === b) {
    return 0;
  }
  return a < b ? -1 : 1;
}

/** The total counts of a list of rows. */
function sumOf(nodes: readonly FileTreeNode[]): DiagnosticCounts {
  return nodes.reduce<DiagnosticCounts>((total, node) => {
    const counts = node.kind === "file" ? node.file.diagnostics : node.diagnostics;
    return {
      error: total.error + counts.error,
      warning: total.warning + counts.warning,
      info: total.info + counts.info,
    };
  }, NONE);
}
