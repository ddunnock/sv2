// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, describe, expect, test } from "bun:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";

import { WorkspacePathSchema } from "@/contract/file";

import { createModelQueries } from "./model-queries";
import { insideTauri, tauriTransport } from "./tauri-client";

/**
 * Pretend to be inside a Tauri window. `mockIPC` replaces `invoke` but does not
 * set the flag Tauri injects into a real window, which `isTauri()` reads.
 */
function enterTauri(answer: (command: string, args: unknown) => unknown): void {
  Reflect.set(globalThis, "isTauri", true);
  mockIPC(answer);
}

afterEach(() => {
  clearMocks();
  Reflect.deleteProperty(globalThis, "isTauri");
});

describe("outside a Tauri window", () => {
  test("there is no core, and the transport says unreachable without calling invoke", async () => {
    let called = false;
    mockIPC(() => {
      called = true;
    });
    expect(insideTauri()).toBe(false);
    expect(await tauriTransport()("workspace", {})).toEqual({ ok: false, error: "unreachable" });
    expect(called).toBe(false);
  });
});

describe("inside a Tauri window", () => {
  test("the command and its arguments reach invoke as given", async () => {
    const sent: unknown[] = [];
    enterTauri((command, args) => {
      sent.push(command, args);
      return null;
    });
    await tauriTransport()("file_text", { path: "model/A.sysml" });
    expect(sent).toEqual(["file_text", { path: "model/A.sysml" }]);
  });

  test("the reply comes back unparsed: checking it is model-queries' job", async () => {
    const odd = { not: "an answer" };
    enterTauri(() => odd);
    expect(await tauriTransport()("workspace", {})).toEqual({ ok: true, value: odd });
  });

  test("a call the core refuses is rejected, and the core's text is not kept", async () => {
    enterTauri(() => {
      throw new Error("Command save_file not allowed by ACL: /Users/someone/secret.sysml");
    });
    const result = await tauriTransport()("save_file", {});
    expect(result).toEqual({ ok: false, error: "rejected" });
    expect(JSON.stringify(result)).not.toContain("secret");
  });

  test("arguments that are not an object of named arguments are not sent", async () => {
    let called = false;
    enterTauri(() => {
      called = true;
    });
    expect(await tauriTransport()("workspace", [1, 2])).toEqual({ ok: false, error: "rejected" });
    expect(called).toBe(false);
  });
});

describe("a Rust-shaped reply through the real parse path", () => {
  // These are what `crates/sv2-studio/src/wire.rs` serializes, written from its
  // serde attributes: tagged `kind`, kebab-case arms, camelCase fields.
  test("workspace", async () => {
    enterTauri(() => ({
      kind: "ready",
      data: {
        name: "thermal",
        files: [
          {
            path: "model/Heater.sysml",
            language: "sysml",
            diagnostics: { error: 1, warning: 0, info: 0 },
          },
        ],
      },
    }));
    const result = await createModelQueries(tauriTransport(), "backend").workspace();
    const files = result.ok && result.value.kind === "ready" ? result.value.data.files : [];
    expect(files.map((file) => String(file.path))).toEqual(["model/Heater.sysml"]);
  });

  test("not-implemented, the answer for everything sv2-resolve would give", async () => {
    enterTauri(() => ({
      kind: "unavailable",
      reason: { kind: "not-implemented", capability: "resolving views" },
    }));
    const result = await createModelQueries(tauriTransport(), "backend").views();
    expect(result.ok && result.value).toEqual({
      kind: "unavailable",
      reason: { kind: "not-implemented", capability: "resolving views" },
    });
  });

  test("not-found, for a path the core refuses to read", async () => {
    const path = WorkspacePathSchema.safeParse("model/Missing.sysml");
    if (!path.success) {
      throw new Error("test path does not satisfy WorkspacePathSchema");
    }
    enterTauri(() => ({ kind: "unavailable", reason: { kind: "not-found" } }));
    const result = await createModelQueries(tauriTransport(), "backend").fileText(path.data);
    expect(result.ok && result.value).toEqual({
      kind: "unavailable",
      reason: { kind: "not-found" },
    });
  });
});
