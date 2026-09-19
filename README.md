# sv2 — scaffold

A Rust workspace for a SysML v2 / KerML front end, packaged with the AI-assisted
development harness it needs to stay honest: hooks that enforce, rules that explain,
and a state file that is mostly derived rather than written.

Rename `sv2` before the first commit if you pick a real project name.

## What is here

```
CLAUDE.md                  invariants and pointers — under 200 lines on purpose
LICENSE                    MIT
.claude/
  settings.json            four hooks — the enforcement layer
  rules/*.md               per-crate invariants, path-scoped so they cost no context
  agents/                  spec-conformance-reviewer, gate-triage
  skills/                  add-production, refresh-snapshots, close-session
  scripts/                 helpers for Claude Code and its skills and agents
    hook-*.sh              the four hook entry points (shims; logic in hook_*.py)
    grammar-preflight.sh   phase 1 of the six-phase grammar derivation
    grammar_*.py           phases 2-6 of the derivation (derive-grammar skill)
    _earley.py             independent validation oracle (no shared code with the parser)
    regen_state.py         rewrite state.json's generated object
    validate_state.py      authored JSON vs. schemas
    state_report.py        human-readable view (prints; writes nothing)
    index_decisions.py     ADR frontmatter -> decisions.json
  state/                   ALL machine-readable work state, as JSON
    state.json             generated (script) + authored (you), schema-validated
    deviations.json        authored: grammar/metamodel deviations with evidence
    coverage.json          derived: production coverage
    grammar-diff.json      derived: Xtext vs specification BNF
    decisions.json         derived: index of docs/adr/
    grammar/*.json         derived from the pinned Xtext
    grammar/units/*.json   one derived production each, fingerprinted
    grammar/reference.json the frozen reference grammar + ledger
    schema/*.json          JSON Schemas for the authored files
scripts/                   project tooling — runs without Claude Code
  gate.sh                  the single definition of "done"
  vendor_sync.py           NETWORK: fetch the pinned vendor set
  vendor_verify.py         offline: vendored files vs. the lockfile
  extract_productions.py   Xtext -> productions / keywords / metaclass-map
  check_namespace_link.py  Xtext metamodel namespace vs. pinned XMI
  check_metaclass_map.py   every rule's metaclass exists in the XMI
  grammar_diff.py          Xtext inventory vs. specification BNF
  bnf_coverage.py          production coverage
  corpus-sweep.sh          positive and negative acceptance
  check_shell_standard.py  STD-003-SH rules that ShellCheck and shfmt cannot express
  check_rust_workspace.py  STD-002-RS manifest, binary-target, and lib.rs rules
  rust_binaries.toml       the crates allowed a binary target (STD-002-RS §2.2)
  check_headers.py         program headers where required; no shebang in Python
  check_standards_config.py config files vs. the standards' normative TOML blocks
  check_rust_patterns.py   STD-002-RS tracing, #[ignore], and banned-name rules
docs/
  adr/                     ten decisions, MADR 4.0.0 — prose, stays markdown
  DERIVATION.md            what can and cannot be derived. Read before assuming.
  conformance-target.toml  the three-tier pin
vendor/                    pinned upstream: omg/ pilot/ corpus/ + sources.lock.toml
crates/                    sv2-syntax, sv2-ast, sv2-hir, sv2-resolve, sv2-cli
tests/corpus, tests/rejection
```

## How the anti-drift layers fit together

They are deliberately layered by strength because the weak ones are the ones people
usually reach for first.

| Layer       | Mechanism                          | Strength                                                            |
| ----------- | ---------------------------------- | ------------------------------------------------------------------- |
| Explanation | `CLAUDE.md`, `.claude/rules/`      | Context. Influences, does not enforce.                              |
| Feedback    | `PostToolUse` hook                 | Surfaces clippy findings. Cannot undo the edit.                     |
| Prevention  | `PreToolUse` hook                  | Denies writes to machine-owned paths, from file tools and Bash.     |
| Completion  | `Stop` hook                        | Refuses to let the turn end while the gate is red.                  |
| Truth       | Grammar pin, coverage gate, corpus | Independent of the agent entirely.                                  |

Claude Code's own documentation is explicit that CLAUDE.md is context rather than enforced
 configuration and directs you to a `PreToolUse` hook when something must be blocked
regardless of what the model decides. The design above takes that literally: the top two
rows explain, the bottom three enforce.

The `Stop` gate is the one worth understanding. A `decision: "block"` from `Stop` forces
the agent to keep working rather than ending its turn, so a red gate is not something that
can be talked past. It honors `stop_hook_active` to avoid looping on a gate that stays red;
if you need to end a turn deliberately with a red gate, `touch .claude/gate-off`.

Hooks answer in JSON rather than with exit 2, so the agent receives only the reason
(STD-003-SH §9.3). The Bash side of the `PreToolUse` guard is a tripwire, not a wall: it
tokenizes each command and denies writes that name a protected path, but a script file or a
variable can still reach one. The gate's `--check` modes catch those after the fact.

## Setup

```bash
git init
# set tier_b_pilot.revision in docs/conformance-target.toml to a full Pilot commit sha
python3.12 scripts/vendor_sync.py          # needs network — the only script that does
python3.12 scripts/extract_productions.py
python3.12 scripts/bnf_coverage.py && python3.12 scripts/grammar_diff.py
python3.12 .claude/scripts/index_decisions.py
python3.12 .claude/scripts/regen_state.py
./scripts/gate.sh
```

Python scripts run on `python3.12` explicitly: on RHEL 9 `/usr/bin/python3` is the 3.9
platform interpreter. Shell scripts follow `docs/standards/STD-003-SH-bash-standards.md`;
Python follows STD-001-PY with the deviations recorded in `pyproject.toml`.

`scripts/` never depends on `.claude/scripts/`, with one exception: `gate.sh` is the union
of every check, so it also runs the frozen-grammar, decisions-index, and state checks that
live beside the Claude Code tooling they serve.

## Why the state is JSON, and why it lives in `.claude/state/`

Everything an agent reads for context or progress is JSON: validatable against a schema,
diffable precisely, and unambiguous about structure. The schemas are not decoration — the
`minLength` on `next_step` exists to stop `"TBD"` from being accepted as a handoff.

`.claude/` is Claude Code's configuration namespace, so the work state gets its own
subdirectory rather than sitting beside `settings.json`. Nothing there loads automatically;
the SessionStart hook is what puts it in context.

Two things stay Markdown because they cannot be otherwise: `CLAUDE.md` and
`.claude/rules/*.md` are the only formats Claude Code reads as instructions, and `docs/adr/`
holds arguments, which do not become more useful as JSON. `decisions.json` indexes the
ADRs; it does not replace them.

Requires `python3.12` (the RHEL 9 AppStream package) for the hooks and scripts. The
protect-paths hook blocks every edit and shell command if it is missing, by design. `cargo-deny` and
`cargo-insta` for the full gate; `shellcheck`, `shfmt`, `ruff`, `mypy`, and `pytest` for
its script checks, which are skipped with a notice when absent.

Open Claude Code at the workspace root and run `/context` to confirm `CLAUDE.md` appears
under Memory files. If it does not, nothing in the top two layers is doing anything.

## Things that will look wrong and are not

- **`sv2` does not build a useful binary yet.** The CLI exits 2 with a pointer to the
  state file. The scripts degrade gracefully around it.
- **Coverage reports a large `unimplemented` count.** That is the honest state of a new
  parser. `unimplemented` is a tracked state; `absent` is the defect.
- **The gate passes with nothing vendored.** The checks are inert until there is something
  to check, and become real the moment you pin. Every one of them was tested against a
  planted failure, because an inert check and a broken check look identical.

## License

MIT — see `LICENSE`. Every source file carries the two-line SPDX and copyright
header, enforced by `scripts/check_headers.py`, so the license and the author travel
with any file copied out of the repository.

Vendored inputs under `vendor/` keep their own licenses: the OMG artifacts under their
published IPR terms, the Pilot Xtext under EPL-2.0 with its notice intact. None of it
is compiled into the crates.

**`NOTICE.md` is the attribution**, and `vendor/sources.lock.toml` is its
machine-readable form — every pinned file with its licence and sha256. The short
version: no specification clause text is in this repository. The grammar derivation
reads the clauses from wikis built locally from OMG's PDFs and records only the
citation, a sha256 of the clause, and what was decided.
`.claude/scripts/check_derivation_text.py` runs on every gate and fails if that line
is crossed.
