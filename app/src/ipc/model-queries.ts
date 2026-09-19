// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The model queries the shell calls, each parsed at the boundary (§4.1).
 *
 * ONE PARSE PATH, WHATEVER IS ON THE OTHER END. A `Transport` moves a command
 * and its arguments to something that answers and hands back what came out,
 * untyped. The fixture client and, in Phase 6, the Tauri client are both
 * transports; everything above them — the schema from `contract/registry.ts`,
 * the `Result`, the error mapping — is this module, and is the same for both.
 * That is what makes a fixture a test of the real path rather than a stand-in
 * for it (§11: fixtures are parsed by the schema the application uses).
 *
 * PROVENANCE IS PART OF THE ANSWER. The status bar shows a persistent "fixture
 * data" indicator, and it reads `provenance` rather than guessing, so a build
 * wired to fixtures cannot be mistaken for one showing a real model.
 */

import type { Answer } from "@/contract/availability";
import type { ElementDetail } from "@/contract/element";
import type { ElementHandle, ViewId } from "@/contract/element-id";
import type { FileText, Workspace, WorkspacePath } from "@/contract/file";
import type { ViewLayout } from "@/contract/layout";
import { COMMANDS, type Command } from "@/contract/registry";
import type { ViewSummary } from "@/contract/view";
import { err, ok, type Result } from "@/model/result";

import { contractError, type IpcError } from "./ipc-error";

/** Why a transport could not deliver a reply. The two cases `IpcError` distinguishes. */
export type TransportFailure = "unreachable" | "rejected";

/**
 * Moves one command to whatever answers it and returns the raw reply.
 *
 * It must not throw: a transport that can fail says so in its `Result`, and
 * the Tauri client catches `invoke`'s rejection to do it. The reply is
 * `unknown` because nothing has checked it yet — that is this module's job.
 */
export type Transport = (
  command: string,
  args: unknown,
) => Promise<Result<unknown, TransportFailure>>;

/** Where answers come from: sample data in the bundle, or the Rust core. */
export type Provenance = "fixture" | "backend";

/** A query's outcome: an answer that was read, or why none could be. */
export type Reply<T> = Promise<Result<Answer<T>, IpcError>>;

/** Every query the shell may ask, and where the answers come from. */
export type ModelQueries = Readonly<{
  provenance: Provenance;
  workspace: () => Reply<Workspace>;
  views: () => Reply<readonly ViewSummary[]>;
  elementDetail: (handle: ElementHandle) => Reply<ElementDetail>;
  viewLayout: (view: ViewId) => Reply<ViewLayout>;
  fileText: (path: WorkspacePath) => Reply<FileText>;
}>;

/** The queries, answered through `transport`. */
export function createModelQueries(transport: Transport, provenance: Provenance): ModelQueries {
  return {
    provenance,
    workspace: () => call(transport, COMMANDS.workspace, {}),
    views: () => call(transport, COMMANDS.views, {}),
    elementDetail: (handle) => call(transport, COMMANDS.elementDetail, { handle }),
    viewLayout: (view) => call(transport, COMMANDS.viewLayout, { view }),
    fileText: (path) => call(transport, COMMANDS.fileText, { path }),
  };
}

/** Sends one command and parses its reply with the command's own schema. */
async function call<A, R>(transport: Transport, command: Command<A, R>, args: A): Reply<R> {
  const sent = await transport(command.command, args);
  if (!sent.ok) {
    return err({ kind: sent.error, command: command.command });
  }
  const parsed = command.answer.safeParse(sent.value);
  if (!parsed.success) {
    return err(contractError(`ipc:${command.command}`, parsed.error));
  }
  return ok(parsed.data);
}
