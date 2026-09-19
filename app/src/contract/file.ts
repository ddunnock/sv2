// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The files of a workspace, as the Files tree reads them (IX-01).
 *
 * RUST COUNTERPART: none yet, and no ADR assigns one. `sv2-syntax` parses one
 * text at a time and knows nothing of files; enumerating a workspace belongs to
 * whichever crate opens one, which ADR-0020 makes the same act as allocating
 * identity. When that crate exists, this is what it is compared against.
 *
 * THE WIRE IS FLAT; THE TREE IS DERIVED. A workspace is a list of files, each
 * with a path, and folders are what those paths have in common. A nested wire
 * shape would state every folder twice — once as a node, once as a prefix of
 * each child's path — and the two could disagree. `model/tree.ts` builds the
 * tree, which is also where sorting belongs.
 *
 * ONLY MODEL FILES CROSS. IX-01 describes a "folder tree of `.sysml`/`.kerml`
 * files". Layout and style sidecars (ADR-0017, `views/<view-id>.*.jsonl`) are
 * machine-written, keyed by view rather than by file, and are not part of what
 * the user navigates. A folder holding no model file therefore does not appear,
 * which is the right answer for a tree of models.
 *
 * NOT HERE YET: whether the workspace can be written. ADR-0020 leaves what a
 * read-only workspace can do as an open question, and its answer is a
 * workspace-level state, not a per-file field.
 *
 * MINTING SITES (§4.4): `mintWorkspacePath` in this module, and nothing else.
 */

import { z } from "zod";

import type { Severity } from "./diagnostic";

/**
 * A file's location within its workspace: relative, `/`-separated, and
 * normalized.
 *
 * ONE FORM, CONVERTED ON THE RUST SIDE, for the same reason offsets are UTF-16
 * there (ADR-0013 RISK-013-2): two spellings of one path would be two keys
 * for one file. So the webview never receives an absolute path, a Windows
 * separator or a `..`, and nothing here converts one.
 *
 * Branded, because a path and a qualified name are both strings and
 * `ThermalControl` is a legal instance of each.
 */
export type WorkspacePath = string & z.core.$brand<"WorkspacePath">;

/**
 * True when `path` is in the one form a workspace path takes.
 *
 * Rejects rather than normalizes, because §4.3 rule 4 allows only a transform
 * that loses nothing, and turning `a/./b` into `a/b` is a claim about what the
 * sender meant.
 */
function isWorkspacePath(path: string): boolean {
  if (path.includes("\\") || path.includes("\0")) {
    return false;
  }
  // A leading "/" gives an empty first segment and a trailing one an empty
  // last, so absolute paths and directory spellings fall out of this check.
  return path.split("/").every((segment) => segment !== "" && segment !== "." && segment !== "..");
}

/**
 * The one function that mints a `WorkspacePath` (§4.4).
 *
 * Reached only from `WorkspacePathSchema`, after `isWorkspacePath` has passed.
 */
function mintWorkspacePath(checked: string): WorkspacePath {
  // biome-ignore lint/nursery/noUnsafeTypeAssertion: the one minting site for WorkspacePath (§4.4); the schema checks the form before reaching here.
  return checked as WorkspacePath;
}

/** A file's location within its workspace. */
export const WorkspacePathSchema: z.ZodType<WorkspacePath, string> = z
  .string()
  .min(1)
  .refine(isWorkspacePath, {
    message: "a workspace path is relative, /-separated, and has no empty, . or .. segment",
  })
  .transform(mintWorkspacePath);

/**
 * The two languages, one grammar each (ADR-0014).
 *
 * Closed, because adding a language is an ADR, not a newer core.
 */
export const LANGUAGES = ["sysml", "kerml"] as const;

/** Which grammar parsed a file. */
export type Language = (typeof LANGUAGES)[number];

/** Which grammar parsed a file. */
export const LanguageSchema: z.ZodType<Language, unknown> = z.enum(LANGUAGES);

/**
 * How many diagnostics a file has, at each severity (IX-10's "count in the
 * explorer").
 *
 * All three are carried though the mockup draws only errors: which to show is
 * a rendering decision, and a count the wire dropped cannot be shown later.
 */
export type DiagnosticCounts = Readonly<Record<Severity, number>>;

// §4.2: u32 or smaller on the wire.
const count = z.number().int().nonnegative().max(0xffff_ffff);

/**
 * How many diagnostics a file has, at each severity.
 *
 * The annotation keeps the keys in step with `Severity`: a fourth severity
 * makes this fail to compile rather than go silently uncounted.
 */
export const DiagnosticCountsSchema: z.ZodType<DiagnosticCounts, unknown> = z.strictObject({
  error: count,
  warning: count,
  info: count,
});

/**
 * One model file.
 *
 * `language` is carried although the extension usually implies it. Which
 * grammar parsed the file is the core's decision, and the S/K badge must show
 * the grammar that was actually used; re-deriving it from the name here would
 * be a second opinion, the same reason severity is carried.
 */
export type WorkspaceFile = Readonly<{
  path: WorkspacePath;
  language: Language;
  diagnostics: DiagnosticCounts;
}>;

/** One model file. */
export const WorkspaceFileSchema: z.ZodType<WorkspaceFile, unknown> = z.strictObject({
  path: WorkspacePathSchema,
  language: LanguageSchema,
  diagnostics: DiagnosticCountsSchema,
});

/**
 * An open workspace: its name and its model files.
 *
 * `name` is what the tree's root row shows — `ThermalControl` in the mockup —
 * and is deliberately not the absolute path of the root, which the webview has
 * no use for.
 */
export type Workspace = Readonly<{
  name: string;
  files: readonly WorkspaceFile[];
}>;

/**
 * True when no two files share a path.
 *
 * Checked once per workspace, the container-level check §4.7 prefers. A
 * duplicate would be two rows with one key in the tree. Paths differing only
 * by case are not duplicates: on a case-sensitive disk they are two files.
 */
function hasUniquePaths(files: readonly WorkspaceFile[]): boolean {
  return new Set(files.map((file) => file.path)).size === files.length;
}

/** An open workspace. */
export const WorkspaceSchema: z.ZodType<Workspace, unknown> = z.strictObject({
  name: z.string().min(1),
  files: z
    .array(WorkspaceFileSchema)
    .readonly()
    .refine(hasUniquePaths, { message: "two files share one workspace path" }),
});
