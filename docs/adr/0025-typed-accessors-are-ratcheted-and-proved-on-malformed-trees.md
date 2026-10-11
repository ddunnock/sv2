---
title: "Typed accessors are ratcheted against the node-kind inventory and proved on malformed trees"
status: proposed
date: 2026-10-11
deciders: [David]
---

# Typed accessors are ratcheted against the node-kind inventory and proved on malformed trees

## Context and problem statement

`sv2-ast` is eight lines of module documentation. The parser below it is finished as a
capability — 553 of 553 grammar units, 311 of 311 corpus files — and what made that build
methodical was not diligence but a denominator: a pinned inventory of productions, a marker
claiming each one, and a report that distinguished *not yet* from *wrong*. Nothing
equivalent exists for the layer above, and without one the crate grows by whichever
accessor the current task happens to need, with no way to say how far along it is.

The denominator is harder here. The parser's came from the specification BNF, pinned and
frozen. The node kinds a tree can contain are **authored by this project** in
`gen_syntax_kinds.py`, so any inventory built from them answers to us rather than to OMG.

The failure mode is different too. The parser's risk was accepting what it should reject;
`unwrap`, `panic` and `indexing_slicing` are already workspace lints, so an accessor is
unlikely to crash. Its risk is **lying**: returning a child that is not there, picking
silently between two children where the grammar allows one, or reporting a name from a
declaration that was never parsed. A tree from malformed text is the normal case for an
editor, not an edge case, and ADR-0002 requires that such elements still exist and carry
their diagnostics. An accessor that is confidently wrong about them defeats that.

There is also an invariant to keep mechanical. ADR-0004 justifies the lossless tree partly
on the grounds that this crate makes it affordable, and that only holds while accessors
stay views: copying text out of the CST into an owned field means the tree has stopped
being the single source of truth.

## Decision drivers

- A new crate must be able to report an honest, large *unimplemented* count without that
  reading as failure — the parser's coverage gate already established this.
- The discipline has to be the default path, not a thing to remember, because the work
  spans many sessions.
- Non-happy-path evidence must be cheap enough to produce for every accessor, or it will
  be skipped for the ones that look obvious.
- Checks that pass because they have nothing to check are worse than no checks.
- Test and development code must not ship inside the crate's API.

## Considered options

1. **Ratchet against the node kinds** (463 authored kinds), marker per accessor, three
   states, failing only on `absent` or staleness.
2. **Ratchet against the frozen grammar's productions** — 554 frozen, of which 553 are the
   live units coverage counts, the difference being the unreachable one ADR-0023 excludes —
   so both layers answer to the pinned specification. Rejected: a production is not
   one-to-one with a node kind, so the gate would need a mapping rule, and a wrong mapping
   would make the count a fiction.
3. **No count at all** — behavioural properties only, accessors appear on demand. Rejected:
   it gives no worklist and no sense of progress, which is the thing that made the parser's
   build tractable across sessions.
4. For the tests: the repository's existing corpora, hand-written malformed cases,
   property tests, and differential assertions — **taken together**, not as alternatives.

## Decision outcome

**The inventory is derived once.** `gen_syntax_kinds.py` emits
`.claude/state/node-kinds.json` beside `kinds.rs` from the list it already owns. Nothing
parses `kinds.rs` a second time; a second reader of generated Rust is how a derived fact
turns into two disagreeing facts.

**Coverage** (`scripts/ast_coverage.py`, in the gate). A `// node: Kind` marker in
`crates/sv2-ast/` claims a typed accessor.

| State | Meaning | Verdict |
|---|---|---|
| implemented | a marker claims a kind in the inventory | counted |
| unimplemented | a kind no marker claims | tracked, never a failure |
| absent | a marker claims a kind that does not exist | defect |

`--check` fails on `absent` or on a stale report. The report is written to
`.claude/state/ast-coverage.json`, machine-owned and protected by the PreToolUse hook. On
the day this lands it reads 0 implemented, 463 unimplemented, 0 absent, which is the honest
state of a new crate and not a red gate.

**The worklist is ordered root downward**, by a breadth-first walk of the frozen grammar
from `RootNamespace`, mapping production names to node kinds where they match. That mapping
is approximate, so the order it produces decides **only how the report prints**. It must
never gate: an ordering that cannot justify itself may not fail anyone's build.

**Owns no data, and no scaffolding ships** (`scripts/check_ast_leaks.py`, in the gate).
Every public accessor type wraps exactly one `SyntaxNode` and nothing else; no `String`,
`Box<str>` or `Vec<String>` field; no public function returns `String`. Accessors hand back
nodes and tokens and the caller asks those for text. Nothing `pub` is named `test_*`,
`dump_*`, `debug_*` or `fixture_*`, and no `pub` item is gated on `#[cfg(test)]`.

**Four kinds of evidence, and an accessor lands with all of them or it does not land.**

| Evidence | Where | What it catches |
|---|---|---|
| Sweep of every tree the repository holds | `tests/sweep.rs` | panics, children that are not children, ranges outside the parent, answers that change between calls |
| A named malformed case per accessor | `tests/malformed.rs`, data in `tests/malformed/` | a child absent, duplicated, or out of order — the conditions the corpora do not happen to contain |
| Property tests over generated input | `tests/properties.rs` | truncation at a multi-byte boundary, nesting near `MAX_DEPTH`, inputs nobody thought to write |
| Differential assertion | inside the two above | an accessor that is confidently **wrong**, which neither a panic check nor an absence check sees |

The sweep reads the 311 corpus files *and* the 299 rejection files. Those rejection files
are malformed by construction, so the hardest inputs for this layer are already in the
repository and grow as the corpora grow. A malformed case proves three things, because
absence is not the only way to be wrong: the parse still yields a tree, the accessor reports
absence rather than guessing, and the neighbouring accessors still answer. Where the grammar
admits one child and the tree carries two, the case pins **which** is chosen.

**Test and development code lives outside the shipped crate.** Unit tests stay at the
bottom of the file they test, in `#[cfg(test)] mod tests`, as STD-002-RS §10 requires — they
are compiled out, and they are the only place a test reaches a private item. Everything else
moves: shared support to `tests/trees/`, data to `tests/malformed/`, tooling to
`examples/`, where `perf_probe` already sets the precedent. The names avoid `utils`,
`helpers`, `common` and the rest of the banned list, which `check_rust_patterns.py` enforces
over every file under `crates/`, test files included.

**The path is the default path.** `.claude/rules/ast.md` carries these rules and loads
whenever `crates/sv2-ast/**` is touched; `.claude/rules/syntax.md` narrows to its own crate.
The `add-accessor` skill walks one accessor end to end: marker, positive case from a real
corpus file, malformed case, differential assertion, coverage, leak lint, tests, gate.

### Consequences

- Good, because the crate reports how far along it is, in an order that means something,
  without a large unimplemented count reading as failure.
- Good, because the hardest test inputs already exist: 299 malformed trees arrive free and
  the set grows with the corpora.
- Good, because "owns no data" and "no test code in the API" become gate failures rather
  than review comments.
- Good, because the layering boundary is written where someone about to cross it is
  reading — `ID-MALFORMED` and `ID-ORPHAN` belong here; allocating an identifier, detecting
  a duplicate, resolving a name and constructing a metaclass do not.
- Bad, because the denominator is ours, not OMG's: adding a node kind to the parser moves
  it, and the report must be regenerated rather than argued with.
- Bad, because the ordering heuristic will be wrong for some kinds, and the temptation will
  be to make it authoritative rather than leave it advisory.
- Bad, because the sweep's cost grows with accessors × trees, and will eventually need
  measuring rather than assuming.
- Bad, because `sv2-syntax`'s round-trip generator cannot be imported across crates, so a
  small generator is duplicated in `tests/trees/`.

### Confirmation

`scripts/tests/test_ast_coverage.py` and `scripts/tests/test_check_ast_leaks.py` cover
every verdict, each positive case with its negative — the rule `.claude/rules/tests.md`
already states.

A check that has nothing to check passes, and this repository has said so about its own
gate. So each check lands **with its negative control run and recorded**: a `// node:`
marker naming a kind that does not exist must report `absent`; a struct carrying a `String`
field must fail; a `pub fn test_*` must fail; an accessor without its marker must fail.

The whole harness is then proved by building **one** accessor through the skill, root
downward, before any others: the marker is claimed, the report moves 0 → 1, the leak lint
holds, and the sweep finds something to sweep. If the path is awkward, that is discovered
at a cost of one accessor rather than fifty.

Changing what `absent` means, or making the ordering gate, is changing this decision. It is
done here, with its reason, never in `ast_coverage.py` alone to clear a gate.
