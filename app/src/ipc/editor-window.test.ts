// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { describe, expect, test } from "bun:test";

import { EDITOR_EVENTS, type EditorHandoff, EditorHandoffSchema } from "@/contract/editor-window";
import { ok } from "@/model/result";

import { createEditorWindow, NO_EDITOR_WINDOW, type WindowEvents } from "./editor-window";
import type { Transport } from "./model-queries";

function handoff(): EditorHandoff {
  const parsed = EditorHandoffSchema.safeParse({
    path: "model/Heater.sysml",
    text: "part def Heater;\n",
    state: { doc: "part def Heater;\n" },
  });
  if (!parsed.success) {
    throw new Error("test handoff does not satisfy EditorHandoffSchema");
  }
  return parsed.data;
}

/** A transport that records what it was sent and answers `reply`. */
function recording(reply: unknown): { transport: Transport; sent: unknown[] } {
  const sent: unknown[] = [];
  return {
    sent,
    transport: (command, args) => {
      sent.push([command, args]);
      return Promise.resolve(ok(reply));
    },
  };
}

const READY_NULL = { kind: "ready", data: null } as const;
const NO_EVENTS: WindowEvents = { subscribe: () => () => undefined };

describe("the commands go through the registry and the one parse path", () => {
  test("undock sends the handoff under editor_undock", async () => {
    const { transport, sent } = recording(READY_NULL);
    const result = await createEditorWindow(transport, NO_EVENTS).undock(handoff());
    expect(sent).toEqual([["editor_undock", { handoff: handoff() }]]);
    expect(result).toEqual({ ok: true, value: READY_NULL });
  });

  test("dock sends the handoff under editor_dock", async () => {
    const { transport, sent } = recording(READY_NULL);
    await createEditorWindow(transport, NO_EVENTS).dock(handoff());
    expect(sent).toEqual([["editor_dock", { handoff: handoff() }]]);
  });

  test("the waiting handoff is parsed with its schema", async () => {
    const { transport } = recording({ kind: "ready", data: handoff() });
    const result = await createEditorWindow(transport, NO_EVENTS).handoff();
    expect(result.ok && result.value.kind === "ready" && result.value.data.text).toBe(
      "part def Heater;\n",
    );
  });

  test("a malformed handoff is a contract error, not an answer", async () => {
    const { transport } = recording({ kind: "ready", data: { path: "model/Heater.sysml" } });
    const result = await createEditorWindow(transport, NO_EVENTS).handoff();
    expect(!result.ok && result.error.kind).toBe("contract");
  });

  test("nothing waiting is not-found, a true answer", async () => {
    const notFound = { kind: "unavailable", reason: { kind: "not-found" } } as const;
    const { transport } = recording(notFound);
    const result = await createEditorWindow(transport, NO_EVENTS).handoff();
    expect(result).toEqual({ ok: true, value: notFound });
  });

  test("focus and requestDock take no arguments", async () => {
    const { transport, sent } = recording(READY_NULL);
    const window = createEditorWindow(transport, NO_EVENTS);
    await window.focus();
    await window.requestDock();
    expect(sent).toEqual([
      ["editor_focus", {}],
      ["editor_request_dock", {}],
    ]);
  });
});

describe("events", () => {
  test("each listener subscribes to its own event by the name Rust emits", () => {
    const subscribed: string[] = [];
    const events: WindowEvents = {
      subscribe: (event) => {
        subscribed.push(event);
        return () => undefined;
      },
    };
    const window = createEditorWindow(recording(null).transport, events);
    window.onDocked(
      () => undefined,
      () => undefined,
    );
    window.onDockRequested(
      () => undefined,
      () => undefined,
    );
    expect(subscribed).toEqual([EDITOR_EVENTS.docked, EDITOR_EVENTS.dockRequested]);
  });
});

describe("outside the desktop app", () => {
  test("there is no second window, and asking for one is unreachable", async () => {
    expect(NO_EDITOR_WINDOW.available).toBe(false);
    expect(await NO_EDITOR_WINDOW.undock(handoff())).toEqual({
      ok: false,
      error: { kind: "unreachable", command: "editor_undock" },
    });
  });
});
