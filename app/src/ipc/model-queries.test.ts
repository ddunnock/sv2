// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { ElementHandleSchema } from "@/contract/element-id";
import { err, ok } from "@/model/result";

import { createModelQueries, type Transport } from "./model-queries";

/** A transport that answers every command with `reply`, recording what it was sent. */
function answering(reply: unknown): { transport: Transport; sent: [string, unknown][] } {
  const sent: [string, unknown][] = [];
  const transport: Transport = (command, args) => {
    sent.push([command, args]);
    return Promise.resolve(ok(reply));
  };
  return { transport, sent };
}

const HEATER = (() => {
  const parsed = ElementHandleSchema.safeParse({ kind: "petname", id: "maple-sunrise-314" });
  if (!parsed.success) {
    throw new Error("fixture does not satisfy ElementHandleSchema");
  }
  return parsed.data;
})();

describe("createModelQueries", () => {
  test("provenance is carried, so the status bar can say where answers come from", () => {
    expect(createModelQueries(answering(null).transport, "fixture").provenance).toBe("fixture");
    expect(createModelQueries(answering(null).transport, "backend").provenance).toBe("backend");
  });

  test("each query sends its Rust command name and camelCase arguments", async () => {
    const { transport, sent } = answering({ kind: "unavailable", reason: { kind: "opening" } });
    const queries = createModelQueries(transport, "backend");
    await queries.workspace();
    await queries.elementDetail(HEATER);
    expect(sent).toEqual([
      ["workspace", {}],
      ["element_detail", { handle: { kind: "petname", id: "maple-sunrise-314" } }],
    ]);
  });

  test("a reply that satisfies the contract comes back ok, parsed", async () => {
    const ready = { kind: "ready", data: { name: "ThermalControl", files: [] } } as const;
    const result = await createModelQueries(answering(ready).transport, "backend").workspace();
    expect(result.ok && result.value).toEqual(ready);
  });

  test("an unavailable answer is ok, not an error: it is a correct reply", async () => {
    const opening = { kind: "unavailable", reason: { kind: "opening" } };
    const result = await createModelQueries(answering(opening).transport, "backend").views();
    expect(result.ok).toBe(true);
  });

  test.each([
    ["unreachable", "unreachable"],
    ["rejected", "rejected"],
  ] as const)("a transport that is %s is reported as such, naming the command", async (_n, why) => {
    const transport: Transport = () => Promise.resolve(err(why));
    const result = await createModelQueries(transport, "backend").workspace();
    expect(result.ok ? null : result.error).toEqual({ kind: why, command: "workspace" });
  });

  describe("a reply that fails its schema", () => {
    // Model text in a malformed reply: what the author wrote, in a place the
    // schema does not allow it.
    const MODEL_TEXT = "Vehicles::Vehicle::engine";
    const malformed = { kind: "ready", data: { name: "W", files: [{ path: MODEL_TEXT }] } };

    test("is a contract error naming the boundary, with each issue's path and code", async () => {
      const result = await createModelQueries(
        answering(malformed).transport,
        "backend",
      ).workspace();
      expect(result.ok).toBe(false);
      if (!result.ok && result.error.kind === "contract") {
        expect(result.error.boundary).toBe("ipc:workspace");
        expect(result.error.issues.length).toBeGreaterThan(0);
        for (const issue of result.error.issues) {
          expect(Object.keys(issue).sort()).toEqual(["code", "path"]);
        }
      } else {
        throw new Error("expected a contract error");
      }
    });

    test("never carries what was received (§7.1, §9.3)", async () => {
      const result = await createModelQueries(
        answering(malformed).transport,
        "backend",
      ).workspace();
      expect(JSON.stringify(result)).not.toContain(MODEL_TEXT);
    });
  });
});
