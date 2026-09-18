// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * A diagnostic, as the Rust core raises it.
 *
 * RUST COUNTERPART: `sv2_syntax::Diagnostic`, whose fields are private and read
 * through `code()`, `severity()`, `range()` and `message()`.
 *
 * SEVERITY IS CARRIED, NOT DERIVED HERE. On the Rust side severity comes from
 * the code — `DiagnosticCode::severity()` — so a caller cannot raise one code at
 * two severities. The webview could re-derive it from the same mapping, and
 * deliberately does not: that would be a second opinion about severity written
 * in a second language, which is the failure ADR-0013 DD-1 rejects for grammars
 * and which has the same shape here. One producer states it; this reads it.
 *
 * THE CODE IS VALIDATED BY SHAPE, NOT ENUMERATED. `DiagnosticCode` is
 * `#[non_exhaustive]` in Rust and today holds only the four `PARSE-*` codes;
 * `HIR-*`, `RES-*` and `ID-*` are planned. A closed enum here would make a
 * webview reject a diagnostic raised by a newer core — dropping the report that
 * something is wrong because the reason for it is unfamiliar, which inverts
 * ADR-0002: decorate by diagnostic state, never filter on it. So the schema
 * checks that a code is namespaced by its raising crate and admits it.
 *
 * A MESSAGE MAY CONTAIN MODEL TEXT. `sv2-cli` renders `unexpected \`x\``, where
 * `x` came out of the user's file. §9.3 forbids reporting model text to a log
 * channel, so `diagnostics/` must never forward one of these messages. It is a
 * value to render, not a value to log, and the thing to log beside it is the
 * code.
 *
 * NOT HERE YET: which file a diagnostic belongs to, and its line and column.
 * `sv2_syntax::Diagnostic` carries neither — it is per-parse, and `LineCol`
 * comes from `OffsetMap` over the text. Both belong to the workspace-level
 * query that returns diagnostics for more than one file, and arrive with it.
 */

import { z } from "zod";

import { type TextSpan, TextSpanSchema } from "./offset";

/**
 * How bad it is, mirroring `sv2_syntax::Severity::as_str`.
 *
 * Closed, unlike the code: a severity the webview does not know has no
 * rendering, so admitting one would buy nothing. Every code raised today is an
 * error — `sv2-syntax` has a test pinning that — and the other two exist
 * because ADR-0002 and ADR-0016 will need them.
 */
export const SEVERITIES = ["error", "warning", "info"] as const;

/** How bad it is. */
export type Severity = (typeof SEVERITIES)[number];

/** How bad it is. */
export const SeveritySchema: z.ZodType<Severity, unknown> = z.enum(SEVERITIES);

/**
 * A namespaced diagnostic code: the raising crate, then what happened.
 *
 * Anchored, uppercase, at least two segments — so `PARSE-EXPECTED` and
 * `PARSE-TOO-DEEPLY-NESTED` both pass and a bare `EXPECTED` does not. The
 * namespace is the part worth checking: it says which crate's vocabulary the
 * rest of the code belongs to, and it is what keeps `sv2-cli`'s process-level
 * `ErrorCode` from ever being mistaken for one of these.
 */
const DIAGNOSTIC_CODE = /^[A-Z]+(?:-[A-Z]+)+$/;

/** A namespaced diagnostic code, such as `PARSE-UNEXPECTED`. */
export const DiagnosticCodeSchema: z.ZodType<string, unknown> = z.string().regex(DIAGNOSTIC_CODE);

/**
 * The codes `sv2-syntax` raises today, from `DiagnosticCode::as_str`.
 *
 * A convenience for a caller that wants to say something specific about one of
 * them, and NOT the set the schema accepts — see the module comment. A code
 * absent from here is still a diagnostic and is still shown.
 */
export const PARSE_CODES = {
  expected: "PARSE-EXPECTED",
  unexpected: "PARSE-UNEXPECTED",
  unterminatedComment: "PARSE-UNTERMINATED-COMMENT",
  tooDeeplyNested: "PARSE-TOO-DEEPLY-NESTED",
} as const;

/** Something the core has to say about a span of a document. */
export type Diagnostic = Readonly<{
  code: string;
  severity: Severity;
  span: TextSpan;
  message: string;
}>;

/**
 * Something the core has to say about a span of a document.
 *
 * The message is not constrained beyond being a string. `sv2-syntax` documents
 * it as a lowercase phrase with no period and no position, but that is an
 * authoring convention on the raising side, and a schema that enforced it would
 * reject a legitimate diagnostic over punctuation — refusing to show the user
 * that their file is broken because the sentence describing it ended wrongly.
 */
export const DiagnosticSchema: z.ZodType<Diagnostic, unknown> = z.strictObject({
  code: DiagnosticCodeSchema,
  severity: SeveritySchema,
  span: TextSpanSchema,
  message: z.string(),
});
