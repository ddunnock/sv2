// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { type ElementHandle, ElementHandleSchema, ViewIdSchema } from "@/contract/element-id";
import { WorkspacePathSchema } from "@/contract/file";

import { fixtureTransport } from "./fixture-client";
import { THERMAL_CONTROL } from "./generated/thermal-control";
import { createModelQueries } from "./model-queries";

/** The fixture, answered through the same path a backend reply takes. */
const queries = createModelQueries(fixtureTransport(THERMAL_CONTROL), "fixture");

function petname(id: string): ElementHandle {
  const parsed = ElementHandleSchema.safeParse({ kind: "petname", id });
  if (!parsed.success) {
    throw new Error("fixture does not satisfy ElementHandleSchema");
  }
  return parsed.data;
}

describe("the ThermalControl fixture satisfies the contract", () => {
  // If any of these fails, the fixture has drifted from the contract — which
  // is the failure a wrong backend reply would produce, caught here first.
  test("workspace", async () => {
    const result = await queries.workspace();
    expect(result.ok && result.value.kind).toBe("ready");
  });

  test("views, all five of the mockup's list", async () => {
    const result = await queries.views();
    expect(result.ok && result.value.kind === "ready" && result.value.data.length).toBe(5);
  });

  test.each(["river-lantern-117", "maple-sunrise-314"])("element %s", async (id) => {
    const result = await queries.elementDetail(petname(id));
    expect(result.ok && result.value.kind).toBe("ready");
  });

  test("Heater keeps its unresolved type and its diagnostic (ADR-0002)", async () => {
    const result = await queries.elementDetail(petname("maple-sunrise-314"));
    const detail = result.ok && result.value.kind === "ready" ? result.value.data : null;
    const state = detail?.ownedFeatures.find((row) => row.name === "state");
    expect(state?.types[0]?.kind).toBe("unresolved");
    expect(detail?.diagnostics).toHaveLength(1);
  });
});

describe("what the fixture will not pretend", () => {
  test("an element it does not describe is not-found, the true answer for this model", async () => {
    const result = await queries.elementDetail(petname("fern-quarry-221"));
    expect(result.ok && result.value).toEqual({
      kind: "unavailable",
      reason: { kind: "not-found" },
    });
  });

  test("every view's layout is not-implemented: no simulated diagrams", async () => {
    const view = ViewIdSchema.safeParse("willow-beacon-551");
    expect(view.success).toBe(true);
    if (view.success) {
      const result = await queries.viewLayout(view.data);
      expect(result.ok && result.value).toEqual({
        kind: "unavailable",
        reason: { kind: "not-implemented", capability: "diagram layout" },
      });
    }
  });
});

describe("file text", () => {
  const path = (value: string) => {
    const parsed = WorkspacePathSchema.safeParse(value);
    if (!parsed.success) {
      throw new Error("fixture does not satisfy WorkspacePathSchema");
    }
    return parsed.data;
  };

  test("the mockup's ThermalControl.sysml is ready, with its line 27", async () => {
    const result = await queries.fileText(path("model/ThermalControl.sysml"));
    const text = result.ok && result.value.kind === "ready" ? result.value.data.text : "";
    expect(text.split("\n")[26]).toBe("        attribute state : HeaterState;");
  });

  test("a file without sample text is not-implemented, not not-found: the file exists", async () => {
    const result = await queries.fileText(path("model/Interfaces.sysml"));
    expect(result.ok && result.value.kind === "unavailable" && result.value.reason.kind).toBe(
      "not-implemented",
    );
  });

  test("Heater's diagnostic span covers HeaterState in the real text", async () => {
    const file = await queries.fileText(path("model/ThermalControl.sysml"));
    const text = file.ok && file.value.kind === "ready" ? file.value.data.text : "";
    const heater = await queries.elementDetail(petname("maple-sunrise-314"));
    const span =
      heater.ok && heater.value.kind === "ready"
        ? heater.value.data.diagnostics[0]?.span
        : undefined;
    expect(span === undefined ? "" : text.slice(span.start, span.end)).toBe("HeaterState");
  });
});

describe("the transport, as a backend would behave", () => {
  const transport = fixtureTransport(THERMAL_CONTROL);

  test("a command the registry does not list is rejected", async () => {
    expect(await transport("delete_everything", {})).toEqual({ ok: false, error: "rejected" });
  });

  test("element_detail with malformed arguments is rejected, not guessed at", async () => {
    expect(await transport("element_detail", { handle: "maple-sunrise-314" })).toEqual({
      ok: false,
      error: "rejected",
    });
  });
});
