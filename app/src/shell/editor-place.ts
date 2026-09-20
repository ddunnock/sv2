// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Where the one open file's editor is, from the main window's side (IX-07).
 *
 * THE EDITOR IS IN ONE PLACE AT A TIME: closed, docked beside the views, or
 * undocked into its own window. Undocked, the main window holds no document
 * at all — only the path, to say where the file went — because the document
 * moved with it (`contract/editor-window.ts`).
 *
 * WHAT THE MAIN WINDOW WILL NOT DO WHILE THE EDITOR IS AWAY. It will not open
 * a second file, which would put two editors in play and leave the returning
 * one nowhere to go; the shell focuses the editor window instead. And it will
 * not close the away editor: that one is closed from its own window, which
 * docks it, so its edits are never dropped from a window that cannot see them.
 */

import type { WorkspacePath } from "@/contract/file";
import type { DocumentSnapshot } from "@/editor/shared-document";

/** Where the editor is. */
export type EditorPlace =
  | Readonly<{ kind: "closed" }>
  /** Beside the views. `seed` is a snapshot that docked back, or `null` to read the disk. */
  | Readonly<{ kind: "docked"; path: WorkspacePath; seed: DocumentSnapshot | null }>
  /** In its own window. */
  | Readonly<{ kind: "undocked"; path: WorkspacePath }>;

/** What changes it. */
export type EditorPlaceAction =
  /** A file was chosen in the Files tree. */
  | Readonly<{ kind: "open"; path: WorkspacePath }>
  /** The docked editor's Close. */
  | Readonly<{ kind: "close" }>
  /** The editor window opened with the file. */
  | Readonly<{ kind: "undocked" }>
  /** The file came back from the editor window. */
  | Readonly<{ kind: "docked"; path: WorkspacePath; snapshot: DocumentSnapshot }>;

/** Nothing open. */
export const NO_EDITOR: EditorPlace = { kind: "closed" };

/** The next place. */
export function editorPlaceReducer(place: EditorPlace, action: EditorPlaceAction): EditorPlace {
  switch (action.kind) {
    case "open":
      if (place.kind === "undocked") {
        return place;
      }
      if (place.kind === "docked" && place.path === action.path) {
        return place;
      }
      return { kind: "docked", path: action.path, seed: null };
    case "close":
      return place.kind === "docked" ? NO_EDITOR : place;
    case "undocked":
      return place.kind === "docked" ? { kind: "undocked", path: place.path } : place;
    case "docked":
      return { kind: "docked", path: action.path, seed: action.snapshot };
  }
}
