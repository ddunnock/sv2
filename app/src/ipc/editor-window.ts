// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Moving the open file between the main window and the editor window (IX-07).
 *
 * The commands go through the one parse path in `model-queries.ts`, exactly
 * as the model queries do. The two events come through `WindowEvents`, a seam
 * like `Transport`: the Tauri implementation is in `tauri-client.ts`, and a
 * test passes its own.
 *
 * OUTSIDE THE DESKTOP APP THERE IS NO SECOND WINDOW. `NO_EDITOR_WINDOW` says
 * so: `available` is false, so the shell disables the control with a reason,
 * and every command answers `unreachable` should one be called anyway.
 */

import { EDITOR_EVENTS, type EditorHandoff } from "@/contract/editor-window";
import { COMMANDS } from "@/contract/registry";
import { err } from "@/model/result";

import type { IpcError } from "./ipc-error";
import { call, type Reply, type Transport } from "./model-queries";

/** Which window this page is: the main window, or the editor's own. */
export type WindowRole = "main" | "editor";

/** Called with the failure when an event cannot be listened for. */
export type ListenFailed = (error: IpcError) => void;

/** Subscribes to an event Rust sends this window. Returns the unsubscribe. */
export type WindowEvents = Readonly<{
  subscribe: (event: string, listener: () => void, failed: ListenFailed) => () => void;
}>;

/** The editor window's commands and events. */
export type EditorWindow = Readonly<{
  /** Whether a second window can exist here. False outside the desktop app. */
  available: boolean;
  /** Main window: move the file into the editor window, opening it. */
  undock: (handoff: EditorHandoff) => Reply<null>;
  /** Editor window: move the file back to the main window, and close. */
  dock: (handoff: EditorHandoff) => Reply<null>;
  /** The file waiting for this window, taken once. */
  handoff: () => Reply<EditorHandoff>;
  /** Main window: bring the editor window forward. */
  focus: () => Reply<null>;
  /** Main window: ask the editor window to dock. */
  requestDock: () => Reply<null>;
  /** Main window: the editor docked, and its file is waiting here. */
  onDocked: (listener: () => void, failed: ListenFailed) => () => void;
  /** Editor window: asked to hand the file back. */
  onDockRequested: (listener: () => void, failed: ListenFailed) => () => void;
}>;

/** The editor window, over `transport` and `events`. */
export function createEditorWindow(transport: Transport, events: WindowEvents): EditorWindow {
  return {
    available: true,
    undock: (handoff) => call(transport, COMMANDS.editorUndock, { handoff }),
    dock: (handoff) => call(transport, COMMANDS.editorDock, { handoff }),
    handoff: () => call(transport, COMMANDS.editorHandoff, {}),
    focus: () => call(transport, COMMANDS.editorFocus, {}),
    requestDock: () => call(transport, COMMANDS.editorRequestDock, {}),
    onDocked: (listener, failed) => events.subscribe(EDITOR_EVENTS.docked, listener, failed),
    onDockRequested: (listener, failed) =>
      events.subscribe(EDITOR_EVENTS.dockRequested, listener, failed),
  };
}

/** Answers every command `unreachable`: there is no core to ask. */
const noCore = (command: string) => (): Reply<null> =>
  Promise.resolve(err({ kind: "unreachable", command }));

/** No second window: a plain browser, or a test that does not need one. */
export const NO_EDITOR_WINDOW: EditorWindow = {
  available: false,
  undock: noCore(COMMANDS.editorUndock.command),
  dock: noCore(COMMANDS.editorDock.command),
  handoff: () =>
    Promise.resolve(err({ kind: "unreachable", command: COMMANDS.editorHandoff.command })),
  focus: noCore(COMMANDS.editorFocus.command),
  requestDock: noCore(COMMANDS.editorRequestDock.command),
  onDocked: () => () => undefined,
  onDockRequested: () => () => undefined,
};
