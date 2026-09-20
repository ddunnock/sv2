// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { EditorHandoffSchema } from "./editor-window";

const handoff = {
  path: "model/ThermalControl.sysml",
  text: "package ThermalControl;\n",
  state: { doc: "package ThermalControl;\n", history: { done: [], undone: [] } },
};

describe("EditorHandoff", () => {
  test("the shape sv2_studio::wire::EditorHandoff serializes is read", () => {
    expect(EditorHandoffSchema.safeParse(handoff).success).toBe(true);
  });

  test("state is carried as it came, not reshaped", () => {
    const parsed = EditorHandoffSchema.safeParse(handoff);
    expect(parsed.success && parsed.data.state).toEqual(handoff.state);
  });

  test("a handoff without state is refused: Rust always sends one", () => {
    const { state: _state, ...withoutState } = handoff;
    expect(EditorHandoffSchema.safeParse(withoutState).success).toBe(false);
  });

  test("state must be JSON, which is all crossing the wire can carry", () => {
    expect(EditorHandoffSchema.safeParse({ ...handoff, state: () => undefined }).success).toBe(
      false,
    );
  });

  test("an unknown field is refused, as Rust's deny_unknown_fields refuses it", () => {
    expect(EditorHandoffSchema.safeParse({ ...handoff, saved: true }).success).toBe(false);
  });

  test("the path is a workspace path", () => {
    expect(EditorHandoffSchema.safeParse({ ...handoff, path: "../escape.sysml" }).success).toBe(
      false,
    );
  });
});
