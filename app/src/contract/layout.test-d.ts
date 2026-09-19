// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Type tests for the grid brand and durable handles (§11 rule 11).
 *
 * Never run; `tsc` checks them. §4.4 names `GridUnit` for one substitution —
 * a pixel value written into a layout record — and both are integers at run
 * time, so only the type system can prove it is prevented.
 */

import type { DurableHandle, ElementHandle } from "./element-id";
import type { GridUnit, NodeRecord } from "./layout";
import type { TextSpan, Utf16Offset } from "./offset";

declare const unit: GridUnit;
declare const pixels: number;
declare const offset: Utf16Offset;
declare const span: TextSpan;
declare const node: NodeRecord;
declare const handle: ElementHandle;

/** §4.4: a pixel value is not a grid unit. */
// @ts-expect-error a plain number, such as a pointer position in pixels, is not in grid units
export const notFromPixels: GridUnit = pixels;

/** Nor is a document offset, the other branded integer. */
// @ts-expect-error Utf16Offset and GridUnit are separately branded
export const notFromOffset: GridUnit = offset;

/** A grid unit is still a number, so the diagram can scale it by the grid size. */
export const stillANumber: number = unit;

/** A dragged node cannot be written back with a pixel position. */
// @ts-expect-error x must be a GridUnit; convert through the grid size first
export const draggedInPixels: NodeRecord = { ...node, x: pixels };

/** A durable handle excludes the transient arm (ADR-0020). */
// @ts-expect-error an unidentified handle cannot key a persisted record
export const noSpanKey: DurableHandle = { kind: "unidentified", span };

/** A general handle is not durable until narrowed. */
// @ts-expect-error ElementHandle includes the unidentified arm
export const notNarrowed: DurableHandle = handle;

/** Narrowing away the one arm is enough. */
export const narrowed: DurableHandle | undefined =
  handle.kind === "unidentified" ? undefined : handle;

/** A durable handle is still a handle. */
export const widens: ElementHandle = node.element;
