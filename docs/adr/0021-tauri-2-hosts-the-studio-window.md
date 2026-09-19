---
title: "Tauri 2 hosts the studio window"
status: accepted
date: 2026-09-19
deciders: [David]
---

# Tauri 2 hosts the studio window

## Context and problem statement

ADR-0018 made `sv2-studio` the long-lived GUI host and left it a stub that exited 2. The
front end in `app/` is built against a contract (`app/src/contract/registry.ts`) and runs
today on fixtures parsed through that contract. Something has to open a window, load
the webview, and answer the registry's commands from Rust.

ADR-0013 already assumes that host is Tauri: its editor reads a syntax tree from a
WebAssembly module "the webview loads", and the whole front end is written for a
webview rather than a native toolkit. What had not been decided is which Tauri, what it
brings into the dependency graph, and what the workspace's supply-chain policy has to
admit to take it.

## Decision drivers

- **DD-1.** The policy is permissive licences only (STD-002-RS §12 rule 5). Anything
  else is an exception, recorded where it is made.
- **DD-2.** Unsafe code is forbidden workspace-wide (`unsafe_code = "forbid"`). The host
  crate has to compile under the same lints as every other crate, not a carve-out.
- **DD-3.** The studio's closure must not leak into the batch command's. ADR-0018 DD-2:
  what `sv2` drags in is what the corpus sweep drags in.
- **DD-4.** No network at runtime (invariant 6). The window must load its front end from
  disk, never from a remote origin.
- **DD-5.** The IPC surface is the registry and nothing more (STD-004-TS §2 rule 3).

## Considered options

1. **Tauri 2.** Stable, the current major version.
2. **Tauri 3.** Only alpha releases exist.
3. **A different webview host** (wry directly, or an Electron-style bundled browser).

## Decision outcome

**Option 1, Tauri 2** — `tauri = "2"` and `tauri-build = "2"` in
`[workspace.dependencies]`, used only by `sv2-studio`, with `serde` (derive) and
`serde_json` for the wire types.

Measured, not assumed: Tauri 2 compiles under the workspace lints including
`unsafe_code = "forbid"` (DD-2), since the forbid applies to this workspace's crates and
not to their dependencies. `sv2-cli` does not depend on it, so the batch command's
closure is unchanged (DD-3).

### What it brings, and what `deny.toml` admits

**Licences.** Three entries join the allowlist, in `deny.toml` and in STD-002-RS §13.5,
which `scripts/check_standards_config.py` keeps identical:

| Licence | Crates | Kind |
|---|---|---|
| Zlib | `foldhash` | Permissive |
| Apache-2.0 WITH LLVM-exception | `target-lexicon` (build tooling) | Permissive |
| **MPL-2.0** | `cssparser`, `cssparser-macros`, `selectors`, `dtoa-short`, `option-ext` | **Weak copyleft — the exception** |

MPL-2.0 is file-level copyleft: its obligations attach to modified MPL-licensed files,
not to the work that links them. All five crates arrive through `tauri-utils`, and none
is modified or vendored, so no obligation attaches to this project's files. Tauri 2
cannot be built without them. This is the one departure from DD-1, and this record is
where it is made.

**`anyhow`.** STD-002-RS §7.1 and §8.3 keep `anyhow` to binaries. Tauri's own crates use
it internally, so `tauri`, `tauri-build` and `tauri-utils` join its `wrappers` list. No
crate of ours gains it.

**Advisories.** `cargo deny check advisories` reported seven findings, all reached only
through Tauri. Each is ignored by ID with a dated reason, accepted on 2026-09-19:

| ID | Crate | Finding | Why accepted |
|---|---|---|---|
| RUSTSEC-2024-0429 | `glib` 0.18 | Unsound iterator impls on `VariantStrIter` | The Linux GTK backend's; nothing here calls that iterator |
| RUSTSEC-2024-0370 | `proc-macro-error` | Unmaintained | Build-time macro under `glib-macros` |
| RUSTSEC-2025-0075, -0080, -0081, -0098, -0100 | `unic-*` (five crates) | Unmaintained | Unicode tables under `urlpattern` ← `tauri-utils` |

The unsound one is the one that matters. It sits in gtk-rs, which Tauri pins; it cannot
be fixed from here, only waited out.

### How the window is configured

- `tauri.conf.json`: the front end is `app/dist` in a build and `http://127.0.0.1:1420`
  in development (`bun run dev`). Both are local (DD-4). The CSP is `default-src 'self'`,
  with `data:` fonts because Bun inlines them (see the UI plan, Phase 1) and IPC as the
  only connection.
- `capabilities/default.json`: `core:default` only. The app's own commands are the
  registry's, registered in `generate_handler!` and nowhere else (DD-5).
- Tauri's generated `gen/` directory is ignored, not committed.

### What the host answers

`sv2-studio` owns two of the registry's commands, and only while nothing below it can:
`workspace` (list the `.sysml` and `.kerml` files under the root, with each file's
diagnostic counts from `sv2-syntax`) and `file_text` (one file's text). Both are
**read-only**. ADR-0020 has opening a workspace allocate identities, which writes; this
host writes nothing until that is built. Both are file access, not model reading, so
they do not break ADR-0018's rule that a binary crate holds no logic about the model.
When a crate below the host loads workspaces — `sv2-resolve` has to, to resolve across
files — ownership moves there and this paragraph is superseded.

The other three commands are registered and answer `not-implemented`, which is the true
answer while `sv2-resolve` is a stub. `view_layout` stays unowned: no record says which
crate reads the layout sidecar.

### Consequences

Good: a real window over the same contract the fixtures satisfy, so no component
changes when the transport does. The policy change is written down in the three places
that must agree — this record, `deny.toml`, and the standard — and the check keeps the
last two from drifting.

Bad: the dependency graph grows by several hundred crates, all under `sv2-studio`. One
MPL-2.0 exception and seven ignored advisories now have to be revisited rather than
forgotten. A debug build of the studio is slow cold.

Neutral: `bundle.active` is false. Packaging an installer is a separate decision, and the
ATO context (ADR-0018 DD-3) will have opinions about it.

## Pros and cons of the options

**Option 1, Tauri 2.** Stable, and what ADR-0013 was written against. Costs the licence
exception and the advisories above.

**Option 2, Tauri 3.** Alpha only. Building a front end on an unreleased major version
moves a stability risk into every other phase for no present gain.

**Option 3, another host.** wry alone would mean reimplementing Tauri's IPC and
capability model, which is the part that enforces DD-5. A bundled browser engine ships a
second runtime and is the opposite of what an air-gapped, audited estate wants.

## More information

### Review triggers

- Tauri or its gtk-rs dependency releases a version that clears RUSTSEC-2024-0429.
- 2027-03-19, for every ignored advisory, whatever else has happened.
- A dependency under Tauri changes licence, or a new MPL-2.0 crate appears anywhere
  outside Tauri's tree: the exception is for Tauri, not for the licence.
- A stable Tauri 3.

### Related decisions

- [ADR-0018](0018-a-binary-per-process-shape.md) — `sv2-studio` as its own process shape.
- [ADR-0013](0013-rust-cst-via-webassembly-as-code-mirror-syntax-tree-source.md) — the
  webview this host loads.
- [ADR-0020](0020-element-identity-is-allocated-when-a-workspace-opens.md) — what opening
  a workspace will eventually write. The host is read-only until it does.
