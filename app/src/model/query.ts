// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The state of one request for an answer, as a component holds it.
 *
 * Three things can be true of a request, and they are one union rather than
 * `isLoading` beside `error` beside `data` (§4.5), which would admit a state
 * that is loading and failed at once.
 *
 * LOADING REMEMBERS WHAT IT HAD. ADR-0002's first consequence is "no flicker
 * during normal editing": the diagram holds still while the user types. A
 * re-request after an edit therefore carries the previous answer, and a panel
 * keeps drawing it until the new one arrives. Only the first request has
 * nothing to show, and that is the one honest place for a loading state.
 *
 * FAILED IS NOT UNAVAILABLE. `failed` holds a defect — the reply could not be
 * read — and is reported. An `unavailable` answer is a correct reply and lives
 * inside `answered`. The error type is a parameter because its definition,
 * `IpcError`, belongs to `ipc/`, which this layer may not import (§2.1).
 */

import type { Answer } from "@/contract/availability";

/** One request's state. `status` because this is internal state (§4.5), not a wire union. */
export type Query<T, E> =
  | Readonly<{ status: "loading"; previous: Answer<T> | null }>
  | Readonly<{ status: "failed"; error: E }>
  | Readonly<{ status: "answered"; answer: Answer<T> }>;

/** A request with nothing yet to show. */
export function firstLoad<T, E>(): Query<T, E> {
  return { status: "loading", previous: null };
}

/**
 * The state after asking again: loading, still holding the last answer.
 *
 * From `failed` there is nothing to hold — a failed read produced no answer —
 * so it behaves as a first load.
 */
export function reload<T, E>(query: Query<T, E>): Query<T, E> {
  switch (query.status) {
    case "answered":
      return { status: "loading", previous: query.answer };
    case "loading":
      return query;
    case "failed":
      return firstLoad();
    default: {
      const unreachable: never = query;
      return unreachable;
    }
  }
}

/**
 * The answer a panel should draw now, if any: the current one, or while
 * reloading, the previous one.
 */
export function answerToShow<T, E>(query: Query<T, E>): Answer<T> | null {
  switch (query.status) {
    case "answered":
      return query.answer;
    case "loading":
      return query.previous;
    case "failed":
      return null;
    default: {
      const unreachable: never = query;
      return unreachable;
    }
  }
}
