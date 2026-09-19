// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { COMMANDS } from "./registry";

const ENTRIES = Object.entries(COMMANDS);

describe("COMMANDS", () => {
  test("no two entries name the same Rust command", () => {
    const names = ENTRIES.map(([, entry]) => entry.command);
    expect(new Set(names).size).toBe(names.length);
  });

  test.each(ENTRIES)("%s is named as a Rust function would be", (_key, entry) => {
    // Tauri invokes a command by its Rust function's name, verbatim.
    expect(entry.command).toMatch(/^[a-z][a-z0-9]*(?:_[a-z0-9]+)*$/);
  });

  test.each(ENTRIES)("%s can answer unavailable, whatever its data", (_key, entry) => {
    // The reason sv2-resolve's absence costs no special path: every command
    // can already say it cannot answer.
    const notImplemented = {
      kind: "unavailable",
      reason: { kind: "not-implemented", capability: "name resolution" },
    };
    expect(entry.answer.safeParse(notImplemented).success).toBe(true);
  });

  test.each(ENTRIES)("%s does not take a bare reply as an answer", (_key, entry) => {
    expect(entry.answer.safeParse({ files: [] }).success).toBe(false);
  });

  test("the owners no ADR assigns are exactly these, so assigning one is a visible change", () => {
    const unassigned = ENTRIES.filter(([, entry]) => entry.owner === "unassigned").map(
      ([key]) => key,
    );
    expect(unassigned.sort()).toEqual(["fileText", "viewLayout", "workspace"]);
  });
});

describe("arguments", () => {
  test("the no-argument commands take an empty object and nothing else", () => {
    expect(COMMANDS.workspace.args.safeParse({}).success).toBe(true);
    expect(COMMANDS.workspace.args.safeParse({ path: "model" }).success).toBe(false);
  });

  test("element_detail takes a handle", () => {
    const args = { handle: { kind: "petname", id: "maple-sunrise-314" } };
    expect(COMMANDS.elementDetail.args.safeParse(args).success).toBe(true);
  });

  test.each([
    ["a bare petname", { handle: "maple-sunrise-314" }],
    ["a qualified name", { handle: "ThermalControl::Heater" }],
    ["no handle", {}],
    ["a stray key", { handle: { kind: "petname", id: "maple-sunrise-314" }, depth: 2 }],
  ])("element_detail rejects %s", (_name, args) => {
    expect(COMMANDS.elementDetail.args.safeParse(args).success).toBe(false);
  });

  test("view_layout takes a view identity, not an element handle", () => {
    expect(COMMANDS.viewLayout.args.safeParse({ view: "amber-lattice-003" }).success).toBe(true);
    const handle = { view: { kind: "petname", id: "amber-lattice-003" } };
    expect(COMMANDS.viewLayout.args.safeParse(handle).success).toBe(false);
  });
});

describe("answers", () => {
  test("views answers a list, and an empty one is ready, not unavailable", () => {
    // A workspace with no views has an answer: none. That is not the same as
    // the core being unable to say.
    expect(COMMANDS.views.answer.safeParse({ kind: "ready", data: [] }).success).toBe(true);
  });

  test("workspace answers the Files tree's shape", () => {
    const ready = { kind: "ready", data: { name: "ThermalControl", files: [] } };
    expect(COMMANDS.workspace.answer.safeParse(ready).success).toBe(true);
  });
});
