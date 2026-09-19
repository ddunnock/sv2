// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Type tests for the workspace path brand (§11 rule 11).
 *
 * Never run; `tsc` checks them. A path and a qualified name are both strings,
 * and `ThermalControl` is a legal instance of each, so no value test can prove
 * they are kept apart. This file does.
 */

import type { SourceLocation } from "./element";
import type { ElementId } from "./element-id";
import type { Workspace, WorkspacePath } from "./file";
import type { TextSpan } from "./offset";

declare const path: WorkspacePath;
declare const plain: string;
declare const id: ElementId;
declare const span: TextSpan;
declare const workspace: Workspace;

/** A string that has not been through WorkspacePathSchema is not a path. */
// @ts-expect-error an unchecked string could be absolute, or use a Windows separator
export const notFromString: WorkspacePath = plain;

/** An element identity is not a path, although both are strings. */
// @ts-expect-error ElementId and WorkspacePath are separately branded
export const notFromElementId: WorkspacePath = id;

/** A path is still a string, so it can be keyed, split and rendered. */
export const stillAString: string = path;

/** The element contract uses the same brand, so there is one path form. */
export const location: SourceLocation = { file: path, span };

// @ts-expect-error an element's location takes a checked path, not any string
export const locationFromString: SourceLocation = { file: plain, span };

/** The file list is readonly; the tree is derived from it, never edited in place. */
// @ts-expect-error Workspace.files is a readonly array
workspace.files.push(workspace.files[0]);
