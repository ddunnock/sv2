// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, describe, expect, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode } from "react";

import { type EditorHandoff, EditorHandoffSchema } from "@/contract/editor-window";
import { type EditorWindow, NO_EDITOR_WINDOW } from "@/ipc/editor-window";
import { fixtureTransport } from "@/ipc/fixture-client";
import { THERMAL_CONTROL } from "@/ipc/generated/thermal-control";
import { createModelQueries } from "@/ipc/model-queries";
import { ok } from "@/model/result";

import { EditorWindowShell } from "./EditorWindowShell";

afterEach(cleanup);

const TEXT = "part def Heater {\n}\n";

function arriving(): EditorHandoff {
  const parsed = EditorHandoffSchema.safeParse({
    path: "model/Heater.sysml",
    text: TEXT,
    state: null,
  });
  if (!parsed.success) {
    throw new Error("test handoff does not satisfy EditorHandoffSchema");
  }
  return parsed.data;
}

/** An editor window that hands over `waiting` once, and records every dock. */
function fake(waiting: EditorHandoff | null): {
  editorWindow: EditorWindow;
  taken: () => number;
  docked: EditorHandoff[];
  requestDock: () => void;
} {
  let takes = 0;
  let left = waiting;
  const docked: EditorHandoff[] = [];
  let onRequest: (() => void) | null = null;
  const editorWindow: EditorWindow = {
    ...NO_EDITOR_WINDOW,
    available: true,
    handoff: () => {
      takes += 1;
      const answer =
        left === null
          ? ({ kind: "unavailable", reason: { kind: "not-found" } } as const)
          : ({ kind: "ready", data: left } as const);
      left = null;
      return Promise.resolve(ok(answer));
    },
    dock: (handoff) => {
      docked.push(handoff);
      return Promise.resolve(ok({ kind: "ready", data: null } as const));
    },
    onDockRequested: (listener) => {
      onRequest = listener;
      return () => {
        onRequest = null;
      };
    },
  };
  return {
    editorWindow,
    taken: () => takes,
    docked,
    requestDock: () => {
      onRequest?.();
    },
  };
}

function page(editorWindow: EditorWindow): void {
  const queries = createModelQueries(fixtureTransport(THERMAL_CONTROL), "fixture");
  render(
    <StrictMode>
      <EditorWindowShell services={{ queries, report: () => undefined, editorWindow }} />
    </StrictMode>,
  );
}

describe("the editor window (SCR-05)", () => {
  test("shows the file it was handed, editable, under its path", async () => {
    page(fake(arriving()).editorWindow);
    const editor = await screen.findByRole("textbox", { name: "model/Heater.sysml" });
    expect(editor.textContent).toContain("part def Heater {");
    expect(screen.getByRole("region", { name: "Editor" }).textContent).toContain(
      "Edits are not saved",
    );
  });

  test("takes the handoff once, although development runs the effect twice", async () => {
    const window = fake(arriving());
    page(window.editorWindow);
    await screen.findByRole("textbox", { name: "model/Heater.sysml" });
    expect(window.taken()).toBe(1);
  });

  test("Dock hands the file back with its path and text", async () => {
    const window = fake(arriving());
    page(window.editorWindow);
    await screen.findByRole("textbox", { name: "model/Heater.sysml" });
    await userEvent.click(screen.getByRole("button", { name: "Dock to main window" }));
    expect(window.docked.map((handoff) => [handoff.path, handoff.text])).toEqual([
      ["model/Heater.sysml", TEXT],
    ]);
  });

  test("a dock request — the main window's Dock, or this window's close — docks it", async () => {
    const window = fake(arriving());
    page(window.editorWindow);
    await screen.findByRole("textbox", { name: "model/Heater.sysml" });
    window.requestDock();
    await Promise.resolve();
    expect(window.docked).toHaveLength(1);
  });

  test("a window with nothing handed to it says so", async () => {
    page(fake(null).editorWindow);
    expect(await screen.findByText(/The file for this window/)).toBeDefined();
  });
});
