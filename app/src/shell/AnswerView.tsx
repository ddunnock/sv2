// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Renders a query in every state it can be in, so no panel handles them twice.
 *
 * - The first load, with nothing yet to show: a status line saying so. That is
 *   the one honest loading state, and it is a sentence, not a spinner.
 * - A reload: the previous answer, still drawn (ADR-0002, no flicker).
 * - A failed read: an alert that it could not be read and has been reported
 *   (§9.2) — never the data it failed on.
 * - An unavailable answer: `<Unavailable>` with the core's reason.
 * - A ready answer: `children`, given the data.
 */

import type { ReactNode } from "react";

import type { IpcError } from "@/ipc/ipc-error";
import { answerToShow, type Query } from "@/model/query";

import { Unavailable } from "./Unavailable";

/** Props for `AnswerView`. */
export type AnswerViewProps<T> = Readonly<{
  query: Query<T, IpcError>;
  /** What is being shown, as the subject of a sentence: "The Files tree". */
  what: string;
  children: (data: T) => ReactNode;
}>;

/** A query, rendered whatever state it is in. */
export function AnswerView<T>({ query, what, children }: AnswerViewProps<T>): ReactNode {
  if (query.status === "failed") {
    return (
      <p role="alert" className="p-4 text-err text-ui">
        {what} could not be read. The failure has been reported.
      </p>
    );
  }
  const answer = answerToShow(query);
  if (answer === null) {
    return (
      <p role="status" className="p-4 text-muted text-ui">
        Loading {what.charAt(0).toLowerCase() + what.slice(1)}…
      </p>
    );
  }
  return answer.kind === "ready" ? (
    children(answer.data)
  ) : (
    <Unavailable what={what} because={answer.reason} />
  );
}
