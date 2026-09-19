// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * A transport that asks the Rust core, through Tauri's `invoke`.
 *
 * The one module that calls `invoke` (STD-004-TS §11 rule 6), and so the one
 * whose tests use `mockIPC`. It moves bytes and nothing else: the reply goes
 * back unparsed, and `model-queries.ts` checks it with the same schema a
 * fixture reply meets.
 *
 * TWO WAYS TO FAIL, KEPT APART. Outside a Tauri window there is no core to
 * ask, which is `unreachable` — the page is running in a plain browser, and
 * `invoke` is never called. Inside one, `invoke` rejects when the core refused
 * the call: the command is not registered, the capability does not grant it,
 * or its arguments did not deserialize. That is `rejected`. The rejection's
 * value is not kept: it is the core's text, and `IpcError` carries none
 * (§9.3).
 */

import { type InvokeArgs, invoke, isTauri } from "@tauri-apps/api/core";

import { err, ok, type Result } from "@/model/result";

import type { Transport, TransportFailure } from "./model-queries";

/** Whether this page is running inside a Tauri window, with a core to ask. */
export function insideTauri(): boolean {
  return isTauri();
}

/** A transport answering from the Rust core. */
export function tauriTransport(): Transport {
  return (command, args) => send(command, args);
}

async function send(command: string, args: unknown): Promise<Result<unknown, TransportFailure>> {
  if (!isTauri()) {
    return err("unreachable");
  }
  if (!isInvokeArgs(args)) {
    // Every registry command takes an object; anything else cannot be sent.
    return err("rejected");
  }
  try {
    return ok(await invoke<unknown>(command, args));
  } catch {
    return err("rejected");
  }
}

/** The one argument shape the registry uses: a plain object of named arguments. */
function isInvokeArgs(args: unknown): args is InvokeArgs {
  return typeof args === "object" && args !== null && !Array.isArray(args);
}
