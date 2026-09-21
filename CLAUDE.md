# sv2 — working agreement

A SysML v2 / KerML front end in Rust: lossless parser, abstract syntax, resolver, and
the `sv2` command. Personal open-source project, MIT.

This file is context, not enforcement. What actually enforces is `scripts/gate.sh`
and the four hooks in `.claude/settings.json`. When this file and a check disagree,
the check is right.

## Before anything else

- **`.claude/state/state.json` is the state of the work.** Objective, next step,
  pending decisions, and the last gate results. `python3.12
  .claude/scripts/state_report.py` renders it. Do not guess what is in progress.
- **`./scripts/gate.sh` is the single definition of done.** Run it before claiming
  anything works. The Stop hook runs it too, so a red gate will not let the turn end.
- **`docs/DERIVATION.md` before any grammar work.** It states what can and cannot be
  derived from the pinned inputs. The common mistake is assuming the abstract syntax
  can produce the concrete syntax; it cannot.

## Invariants

These are the claims the whole design rests on. Breaking one is a defect even when
every test passes.

1. **The parse tree is lossless.** `parse(s).text() == s` for every input, including
   whitespace, both comment forms, and the author's spacing. A graphical edit
   downstream must produce a minimal text delta, and it cannot if the tree threw
   formatting away. ADR-0004.
2. **The IR admits everything that parses.** Elements with unresolved references or
   failed constraints still enter the IR carrying their diagnostics. Validity gates
   writes, never reads. ADR-0002.
3. **The parser does not panic on any input.** Not on malformed text, not on
   truncated text, not on a multi-byte character boundary. `unwrap`, `expect`,
   `panic!`, indexing and `str` slicing are all lints in the workspace lint table.
4. **Every claim about the language traces to a source.** A clause citation, a corpus
   file, or a recorded deviation in `.claude/state/deviations.json`. A plausible guess
   that parses is the most expensive thing you can write here, because it looks right.
5. **Text is authoritative; diagrams are projections.** ADR-0001.
6. **No network at runtime, ever.** This has to work air-gapped. `vendor_sync.py` is
   the only script that fetches, and it is never called from a build or a gate.

## Layout

```
crates/sv2-syntax     lossless CST over rowan, no I/O          .claude/rules/syntax.md
crates/sv2-ast        typed accessors over CST nodes           .claude/rules/syntax.md
crates/sv2-hir        desugaring, implied specialization       .claude/rules/hir.md
crates/sv2-resolve    libraries, names, derived props          .claude/rules/resolve.md
crates/sv2-cli        the `sv2` binary
scripts/              project tooling, runs without Claude Code
.claude/scripts/      hooks, skills, and grammar derivation
.claude/state/        all machine-readable work state, as JSON
docs/adr/             ten decisions, MADR — the arguments, not the summaries
docs/standards/       STD-001-PY, STD-002-RS, STD-003-SH
```

Each crate depends only on the ones above it in that list. `deny.toml` enforces the
direction; a new crate fails `cargo deny check bans` until its layer is declared.

The `.claude/rules/*.md` files are path-scoped: they load when you touch the matching
files, and they carry the detail this file deliberately does not.

## Machine-owned paths

The PreToolUse hook blocks writes to these. If one looks wrong, the generator or the
pinned input is wrong — never the file.

- `vendor/**` — pinned by sha256 and OMG File ID. Moving to a new upstream is
  `python3.12 scripts/vendor_sync.py --accept-new`, a deliberate act, recorded in
  `.claude/state/deviations.json`.
- `.claude/state/grammar/**`, `coverage.json`, `grammar-diff.json`, `decisions.json` —
  derived. Regenerate; do not edit.

## Working rules

- **Fix the rule, never the check.** The checkers are deterministic. Widening an
  allowed set or editing a checker to clear a gate is the one move that makes every
  other guarantee in this repository worthless.
- **Never weaken a failing test.** Not by narrowing an assertion, not with
  `#[ignore]`, not by deleting the case. If the test is genuinely wrong, say so, cite
  what the spec requires, and fix it as its own change.
- **Snapshots one at a time.** `cargo insta review`, never `accept`. If a snapshot
  moved and you cannot name the code change that moved it, stop.
- **Expectations come from the spec or the corpus, never from output.** A test written
  from what the code printed proves only that the code does what it does.
- **Add the negative case in the same change as the positive one.** A parser that
  accepts everything passes a positive-only sweep.
- **Mark parser functions with `// production: <Name>`.** The coverage gate reads
  them. Leaving a production unmarked is honest; marking one that does not exist in
  the inventory fails the gate.
- **Cite the clause at the site** for an implied specialization, a desugaring, or a
  constraint (`// constraint: Type::no_cyclic_specialization`).

## Standards

`docs/standards/` governs code style and enforcement, and the configuration in each
document's enforcement section is normative — `scripts/check_standards_config.py`
fails if the repository and the standard disagree. Rust is STD-002-RS, Python
STD-001-PY, Bash STD-003-SH.

Every source file opens with the two-line license header:

```
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
```

`#` comments in Python and shell, after the shebang where there is one.
`scripts/check_headers.py` enforces it.

## Skills

- `add-production` — one production end to end: parser, CST node, snapshot, positive
  and negative tests, coverage marker.
- `derive-grammar` — the six-phase derivation from the pinned inputs.
- `refresh-snapshots` — review pending snapshots one at a time, with a reason each.
- `close-session` — run the gates, regenerate the derived state, write the authored
  block, append the log entry.

## Things that look wrong and are not

- **Coverage reports a large `unimplemented` count.** That is the honest state of a
  new parser. `absent` is the defect; `unimplemented` is a tracked state.
- **The gate passes with nothing vendored.** The checks are inert until there is
  something to check and become real the moment you pin.
- **`sv2` exits 2 for everything except `--version`.** No command is implemented yet,
  and the corpus sweep reads a non-zero exit as "did not parse", which is the honest
  answer until the parser exists.
