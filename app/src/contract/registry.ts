// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * Every IPC command the webview may call, with the shapes that cross (§4.2).
 *
 * THIS IS WHERE THE CONTRACT IS COLLECTED. `tools/emit-contract.ts` reads it to
 * write `ts.schema.json`, which `scripts/check_ipc_contract.py` compares with
 * the Rust side's; `capabilities/default.json` must allow exactly these
 * commands (§2 rule 3); and `ipc/` looks commands up here rather than naming a
 * string at a call site. A command not listed here is a boundary with no check.
 *
 * NO COMMAND EXISTS IN RUST YET. `sv2-studio` is a stub that exits 2. So this
 * list states the queries the shell needs, in the shape the Rust side must
 * answer, and each entry names its owner — the standing rule that a field with
 * no named Rust owner is a defect. Where no ADR assigns one, the owner is
 * `unassigned`, which keeps the gap searchable instead of filled with a guess.
 *
 * THE NAMES ARE RUST'S. A Tauri command is invoked by its Rust function's name,
 * so `command` is snake_case; arguments and answers follow §4.2's camelCase,
 * which Tauri converts. Every answer is an `Answer`, so `unavailable` has one
 * path from the first fixture to the last query that retires it.
 *
 * NOT HERE YET: the Elements tree (IX-01's second mode), the editor's file
 * text, the Problems panel, and anything that writes. Each arrives with the
 * phase that renders it.
 */

import { z } from "zod";

import { type Answer, answerSchema } from "./availability";
import { type ElementDetail, ElementDetailSchema } from "./element";
import { type ElementHandle, ElementHandleSchema, type ViewId, ViewIdSchema } from "./element-id";
import { type Workspace, WorkspaceSchema } from "./file";
import { type ViewLayout, ViewLayoutSchema } from "./layout";
import { type ViewSummary, ViewSummarySchema } from "./view";

/**
 * The crate that answers a command.
 *
 * `sv2-resolve` owns anything resolved. `unassigned` is honest: no ADR says
 * which crate enumerates a workspace or reads a sidecar.
 */
export type RustOwner = "sv2-resolve" | "unassigned";

/** One command: its Rust name, what it takes, what it answers, and who answers it. */
export type Command<A, R> = Readonly<{
  command: string;
  owner: RustOwner;
  args: z.ZodType<A, unknown>;
  answer: z.ZodType<Answer<R>, unknown>;
}>;

/** The arguments of a command that takes none. */
export type NoArgs = Readonly<Record<string, never>>;

const noArgs: z.ZodType<NoArgs, unknown> = z.strictObject({});

/** Every command, keyed by how the webview names it. */
export type Commands = Readonly<{
  workspace: Command<NoArgs, Workspace>;
  views: Command<NoArgs, readonly ViewSummary[]>;
  elementDetail: Command<Readonly<{ handle: ElementHandle }>, ElementDetail>;
  viewLayout: Command<Readonly<{ view: ViewId }>, ViewLayout>;
}>;

/**
 * Every command.
 *
 * `args` schemas are what `emit-contract` publishes for the comparison; the
 * webview builds its arguments from values that are already typed, so it does
 * not parse them. `answer` schemas are what `ipc/` parses every reply with.
 */
export const COMMANDS: Commands = {
  /** The Files tree (IX-01). */
  workspace: {
    command: "workspace",
    owner: "unassigned",
    args: noArgs,
    answer: answerSchema(WorkspaceSchema),
  },
  /** The Views list (UI-03). */
  views: {
    command: "views",
    owner: "sv2-resolve",
    args: noArgs,
    answer: answerSchema(z.array(ViewSummarySchema).readonly()),
  },
  /** The Specification tab (IX-05) for one element. */
  elementDetail: {
    command: "element_detail",
    owner: "sv2-resolve",
    args: z.strictObject({ handle: ElementHandleSchema }),
    answer: answerSchema(ElementDetailSchema),
  },
  /** One view's layout sidecar (ADR-0017). */
  viewLayout: {
    command: "view_layout",
    owner: "unassigned",
    args: z.strictObject({ view: ViewIdSchema }),
    answer: answerSchema(ViewLayoutSchema),
  },
};
