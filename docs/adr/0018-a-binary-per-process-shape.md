---
title: "A binary per process shape, and every binary a shim"
status: accepted
date: 2026-09-18
deciders: [David]
---

# A binary per process shape, and every binary a shim

## Context and problem statement

STD-002-RS opened §3 with a sentence of fact that had quietly become a constraint:

> `sv2` is the workspace's only binary.

It was true when it was written and it stopped being true the moment the end state was
taken seriously. That end state is a Tauri application: a window hosting a diagram
renderer and a text editor, with edits flowing both ways through the IR, and validity
gating writes (ADR-0001, ADR-0002). Building it produces a GUI host process. ADR-0013
had already committed to a second artifact besides — a WebAssembly module the webview
loads, so that the editor's syntax tree comes from the same Rust parser as everything
else rather than from a second grammar maintained in Lezer.

So the workspace was always going to emit more than one thing. The question is what rule
replaces "one binary", because the rule §2.2 states — *only crates listed in
`scripts/rust_binaries.toml` may have a binary target* — is a good one and is not the
same claim.

## Decision drivers

- **DD-1.** §3 is a *batch command* contract: one `run` function over injected streams,
  one exit status per error code, no work before the request is known, tested without a
  subprocess. It is worth keeping intact and it cannot describe a GUI.
- **DD-2.** `cargo deny` evaluates bans, licences and advisories over the whole
  dependency graph. What the batch command drags in is what CI and the corpus sweep
  drag in.
- **DD-3.** This repository feeds an air-gapped, ATO-governed delivery estate. A small
  headless tool is easier to pin, audit and justify than the same binary with a browser
  engine linked into it.
- **DD-4.** ADR-0013 RISK-013-4 requires that library-wide resolution not run in the
  webview. Whatever the rule is, it must make that structural rather than remembered.
- **DD-5.** §2.2's real protection — that a *library* never gains a second, undisciplined
  path into it — must survive whatever replaces the counting.

## Considered options

1. **One binary with modes.** `sv2 studio` starts the window; `sv2 parse` stays a batch
   command.
2. **A binary per process shape.** One entry point per kind of process, each a shim over
   library crates.
3. **A binary per feature.** `sv2-parse`, `sv2-check`, `sv2-fmt`, `sv2-studio`.

## Decision outcome

**Option 2.** A binary exists per *process shape*, and every binary is a shim.

Three shapes are foreseen, and two exist:

| Binary | Shape | Contract |
|---|---|---|
| `sv2` (`sv2-cli`) | Batch command: read arguments, do work, exit with a status | STD-002-RS §3 |
| `sv2-studio` | Long-lived GUI host: a window, a webview, an event loop | Its own; §3 does not apply |
| `sv2-lsp` | Language server: a long-lived process speaking LSP on a pipe | Not yet written |

`sv2-wasm` is **not** on this list. It is a `cdylib` loaded by the webview, not a binary,
and `scripts/check_rust_workspace.py` only considers targets of kind `bin`, so it never
touches the allowlist.

**Every binary is a shim.** `main` contains a call and nothing else; the behaviour lives
in the crate's library target so it can be tested without spawning a process. This is
what preserves DD-5: what §2.2 protects against is a *library* acquiring an entry point
nobody has described, and that protection is about discipline at the entry point, not
about how many entry points there are. A binary that held logic would be the defect,
whether it were the first or the third.

**The layering rule is unchanged and now written down.** A crate may depend on any layer
below it and never on one above. Two edges are named explicitly because both are easy to
add by accident and neither is obvious:

- **`sv2-wasm` may reach `sv2-syntax` and nothing else.** This is ADR-0013 RISK-013-4
  made structural: the adapter cannot reach the resolver because it appears in no other
  wrapper list, so `cargo deny check bans` rejects the edge. The standard library's
  memory footprint stays out of the webview by construction.
- **Nothing may depend on `sv2-wasm`.** It is an artifact the webview loads, built for
  `wasm32-unknown-unknown`. Linking it into `sv2-studio` would put a second parser in
  the same process as the first, which is exactly the duplication ADR-0013 exists to
  avoid.

### Consequences

Good: §3 stays a precise contract for the thing it actually describes, rather than being
widened until it describes nothing. The batch command keeps a dependency closure that a
reviewer can read, which is DD-2 and DD-3. The webview's exclusion of the resolver is
enforced by the dependency graph rather than by discipline, which is DD-4.

Bad: the workspace has more than one entry point to keep honest, and the shim rule is a
review rule at each of them rather than a check. `scripts/rust_binaries.toml` needs an
entry per binary, and the layering needs a row in `deny.toml` and in STD-002-RS §13.5,
which `scripts/check_standards_config.py` diffs byte for byte. That diff is the point:
adding a crate means deciding which layer it is in, and the change to those two files is
where that decision is reviewed.

Neutral: `sv2-studio` has its own exit statuses and they are not §3's taxonomy. §3 maps
one status per `ErrorCode` because `scripts/corpus-sweep.sh` reads the status alone and
must not confuse "did not parse" with "could not be opened". A window that failed to
open is not that kind of answer, and sharing the enum would tie a GUI's failures to a
batch tool's contract.

### One rule could not be checked where it belongs

`deny.toml` cannot express "nothing may depend on this crate". An empty `wrappers` list
bans the crate outright, and since `sv2-wasm` is a workspace member the check then fails
on the crate's own existence rather than on any edge. That was tried, and it failed that
way.

The rule is therefore checked by `scripts/check_rust_workspace.py`, which reads the same
`cargo metadata` and reports any member depending on a leaf crate. Recording this here
rather than silently splitting the enforcement is the point: the layering is described
in one table in §2.5, and a reader of that table needs to know that one of its rows is
enforced somewhere else.

## Pros and cons of the options

**Option 1, one binary with modes.** Good, because the allowlist stays at one entry and
`sv2 --version` remains the single thing to run. Bad, because §3 would have to describe
both shapes: either its exit-code taxonomy and its "no work before the request is known"
rule apply to a window, which they cannot, or they are qualified until they constrain
nothing. Bad, because the batch command's dependency closure becomes the GUI's, which
DD-2 and DD-3 both refuse.

**Option 2, a binary per process shape.** The chosen option. It costs an allowlist entry
and a layering row per binary, and it keeps each contract about one kind of process.

**Option 3, a binary per feature.** Good, because each binary would be trivially small.
Bad, because the shapes are what differ, not the features: `parse`, `check` and `fmt` are
one contract with three subcommands, and splitting them multiplies entry points without
separating anything. Rejected as the failure mode Option 2 is trying to avoid, arrived
at from the other side.

## More information

### What this record does not decide

The crate decomposition beyond the binaries. `docs/CRATES.md` proposes `sv2-parser`,
`sv2-view`, `sv2-sidecar` and `sv2-edit` and specifies interfaces for all four. None of
them exists, none of their requirements has been exercised, and `state.json` still
carries `crate-split` as an open question about the *one* boundary between crates that
do exist. Fixing four more interfaces from a document written in isolation would decide
more than has been learned. This record fixes the rule; the roster is decided a crate at
a time, as each is written.

### Review triggers

- A third process shape appears that is neither batch, GUI, nor language server.
- A binary starts to hold logic, which is the defect this record's second half exists to
  prevent.

### Related decisions

- [ADR-0013](0013-rust-cst-via-webassembly-as-code-mirror-syntax-tree-source.md) — the
  webview's syntax tree comes from this workspace's parser, which is what makes
  `sv2-wasm` an artifact and RISK-013-4 an edge in the graph.
- [ADR-0001](0001-text-is-authoritative.md) — the round trip the studio exists to close.
- [ADR-0002](0002-ir-admits-what-parses.md) — validity gates writes and never reads,
  which is why the studio renders from the IR rather than from what validates.
