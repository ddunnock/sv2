// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * A transport that answers from sample data in the bundle, until the Rust
 * core can (Phase 6 replaces it with the Tauri client).
 *
 * It returns raw, unparsed replies, exactly as a backend would, so every
 * fixture goes through `model-queries.ts`'s schema check. A fixture that does
 * not satisfy the contract fails there, loudly, the same way a wrong backend
 * reply would.
 *
 * WHAT IT WILL NOT PRETEND. Layout is answered `not-implemented` for every
 * view, because the plan forbids simulated diagrams: an invented layout would
 * read as "the diagram works", which is false. An element the fixture does not
 * describe is `not-found`, which is the true answer for a model that does not
 * contain it.
 */

import { COMMANDS } from "@/contract/registry";
import { logLabel } from "@/model/element-handle";
import { err, ok, type Result } from "@/model/result";

import type { Transport, TransportFailure } from "./model-queries";

/**
 * Sample replies, as raw JSON-shaped values.
 *
 * `elements` is keyed by `logLabel` of the handle — a petname for the elements
 * a fixture describes — because a handle is a structure and a map needs a key.
 */
export type Fixtures = Readonly<{
  workspace: unknown;
  views: unknown;
  elements: Readonly<Record<string, unknown>>;
}>;

const NOT_FOUND = { kind: "unavailable", reason: { kind: "not-found" } } as const;

const NO_LAYOUT = {
  kind: "unavailable",
  reason: { kind: "not-implemented", capability: "diagram layout" },
} as const;

/** A transport answering from `fixtures`. */
export function fixtureTransport(fixtures: Fixtures): Transport {
  return (command, args) => Promise.resolve(reply(fixtures, command, args));
}

/** The raw reply to one command, as a backend would send it. */
function reply(
  fixtures: Fixtures,
  command: string,
  args: unknown,
): Result<unknown, TransportFailure> {
  switch (command) {
    case COMMANDS.workspace.command:
      return ok(fixtures.workspace);
    case COMMANDS.views.command:
      return ok(fixtures.views);
    case COMMANDS.elementDetail.command:
      return elementReply(fixtures, args);
    case COMMANDS.viewLayout.command:
      return ok(NO_LAYOUT);
    default:
      // A command the registry does not list, which a real backend would refuse.
      return err("rejected");
  }
}

/** The fixture for the element `args` names, or `not-found`. */
function elementReply(fixtures: Fixtures, args: unknown): Result<unknown, TransportFailure> {
  // A backend checks its own arguments; this stands in for one, so it does too.
  const parsed = COMMANDS.elementDetail.args.safeParse(args);
  if (!parsed.success) {
    return err("rejected");
  }
  return ok(fixtures.elements[logLabel(parsed.data.handle)] ?? NOT_FOUND);
}
