// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The `Result` type every boundary returns.
 *
 * `model` is pure TypeScript and imports nothing from outside the project
 * (STD-004-TS §2.1), so this file is the bottom of the dependency matrix and can
 * be used from every layer above it.
 *
 * A boundary that can fail returns a `Result` rather than throwing, because a
 * thrown value has no type and the compiler cannot make a caller handle it. The
 * `ipc` and `wasm` layers both parse a wire shape and hand back one of these;
 * `diagnostics` is what turns the failures into something a person reads.
 */

/** A value that arrived and passed its check. */
export type Ok<T> = { readonly ok: true; readonly value: T };

/** A failure, carrying why rather than a thrown value with no type. */
export type Err<E> = { readonly ok: false; readonly error: E };

/** The outcome of anything that crosses a boundary. */
export type Result<T, E> = Ok<T> | Err<E>;

/** Wrap a value that succeeded. */
export function ok<T>(value: T): Ok<T> {
  return { ok: true, value };
}

/** Wrap a failure. */
export function err<E>(error: E): Err<E> {
  return { ok: false, error };
}

/**
 * Narrow a `Result` to its success arm.
 *
 * A function rather than a bare `result.ok` test so that the narrowing has one
 * definition; `exactOptionalPropertyTypes` and `noUncheckedIndexedAccess` make
 * ad-hoc narrowing easy to get subtly wrong.
 */
export function isOk<T, E>(result: Result<T, E>): result is Ok<T> {
  return result.ok;
}
