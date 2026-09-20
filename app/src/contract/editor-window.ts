// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * An open file moving between the main window and the editor window (IX-07).
 *
 * Mirrors `sv2_studio::wire::EditorHandoff`.
 *
 * THE FILE MOVES; IT IS NEVER COPIED. Two windows are two JavaScript
 * contexts, so the editor cannot be two views over one document across them.
 * It lives in one window at a time, and moving it sends this: the path, the
 * text as last edited (nothing saves yet, so it may differ from the disk), and
 * the editor's own serialized state, undo history included.
 *
 * `state` IS OPAQUE HERE. It is whatever `editor/` wrote, carried through Rust
 * untouched, and only `editor/` reads it back. The schema requires only that
 * it is JSON, which is all that crossing the wire can promise; if `editor/`
 * cannot restore it, it starts from `text` and loses only the undo history.
 */

import { z } from "zod";

import { type WorkspacePath, WorkspacePathSchema } from "./file";

/**
 * The events Rust sends about the editor window, by the names
 * `sv2_studio::editor_window` emits. Neither carries a payload: the file itself
 * is fetched with `editor_handoff`, through the one parse path.
 */
export const EDITOR_EVENTS = {
  /** To the main window: the editor docked, and a handoff is waiting. */
  docked: "editor-docked",
  /** To the editor window: hand the file back (Dock, or the window's close button). */
  dockRequested: "editor-dock-requested",
} as const;

/** A file in transit between windows. */
export type EditorHandoff = Readonly<{
  path: WorkspacePath;
  text: string;
  /** The editor's serialized state. Read only by `editor/`. */
  state: unknown;
}>;

/** A file in transit between windows. */
export const EditorHandoffSchema: z.ZodType<EditorHandoff, unknown> = z.strictObject({
  path: WorkspacePathSchema,
  text: z.string(),
  state: z.json(),
});
