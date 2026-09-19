// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { answerSchema, UnavailableReasonSchema } from "./availability";
import { WorkspaceSchema } from "./file";
import { ViewSummarySchema } from "./view";

// A real query's data, so the generic path is exercised with a schema the
// shell will actually wrap: the Files tree, and a view.
const WorkspaceAnswer = answerSchema(WorkspaceSchema);
const ViewAnswer = answerSchema(ViewSummarySchema);

const WORKSPACE = {
  name: "ThermalControl",
  files: [
    {
      path: "model/ThermalControl.sysml",
      language: "sysml",
      diagnostics: { error: 1, warning: 0, info: 0 },
    },
  ],
} as const;

/** What almost every query answers today, because sv2-resolve is a stub. */
const NOT_IMPLEMENTED = {
  kind: "unavailable",
  reason: { kind: "not-implemented", capability: "name resolution" },
} as const;

describe("UnavailableReasonSchema", () => {
  test.each([
    [
      "not-implemented, naming what is missing",
      { kind: "not-implemented", capability: "name resolution" },
    ],
    ["no-workspace", { kind: "no-workspace" }],
    ["opening (ADR-0020)", { kind: "opening" }],
    ["read-only (ADR-0020)", { kind: "read-only" }],
    ["not-found", { kind: "not-found" }],
  ])("%s is a reason", (_name, value) => {
    expect(UnavailableReasonSchema.safeParse(value).success).toBe(true);
  });

  test.each([
    // Deliberately not reasons; see the module comment for each.
    ["parse-failed, since the parser admits everything (ADR-0002)", { kind: "parse-failed" }],
    ["unsupported-language, since only two exist (ADR-0014)", { kind: "unsupported-language" }],
    ["not-implemented without saying what", { kind: "not-implemented" }],
    ["not-implemented with an empty capability", { kind: "not-implemented", capability: "" }],
    ["a reason carrying a message to show", { kind: "not-found", message: "gone" }],
    ["a bare string", "not-implemented"],
  ])("%s is not a reason", (_name, value) => {
    expect(UnavailableReasonSchema.safeParse(value).success).toBe(false);
  });
});

describe("answerSchema", () => {
  test("a ready answer parses, and its data is the wrapped schema's output", () => {
    const result = WorkspaceAnswer.safeParse({ kind: "ready", data: WORKSPACE });
    expect(result.success).toBe(true);
    expect(result.success && result.data.kind === "ready" && result.data.data.name).toBe(
      "ThermalControl",
    );
  });

  test("an unavailable answer parses for any data type, with no data", () => {
    expect(WorkspaceAnswer.safeParse(NOT_IMPLEMENTED).success).toBe(true);
    expect(ViewAnswer.safeParse(NOT_IMPLEMENTED).success).toBe(true);
  });

  test("ready data is checked by the wrapped schema, not waved through", () => {
    const badPath = {
      ...WORKSPACE,
      files: [{ ...WORKSPACE.files[0], path: "/abs/ThermalControl.sysml" }],
    };
    expect(WorkspaceAnswer.safeParse({ kind: "ready", data: badPath }).success).toBe(false);
  });

  test("the same payload is ready for one query and wrong for another", () => {
    // The data schema is per query, so a workspace is not a view.
    expect(ViewAnswer.safeParse({ kind: "ready", data: WORKSPACE }).success).toBe(false);
  });

  test.each([
    ["ready with no data", { kind: "ready" }],
    ["ready with null data, which is not the same as unavailable", { kind: "ready", data: null }],
    ["unavailable with no reason", { kind: "unavailable" }],
    ["both at once", { kind: "ready", data: WORKSPACE, reason: NOT_IMPLEMENTED.reason }],
    ["unavailable still carrying data", { ...NOT_IMPLEMENTED, data: WORKSPACE }],
    ["the plan's status tag instead of §4.2's kind", { status: "ready", data: WORKSPACE }],
    ["a loading state, which is the opening reason, not a third arm", { kind: "loading" }],
  ])("%s is not an answer", (_name, value) => {
    expect(WorkspaceAnswer.safeParse(value).success).toBe(false);
  });
});
