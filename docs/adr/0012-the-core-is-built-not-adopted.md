---
title: "The core is built, not adopted"
status: proposed
date: 2026-09-19
deciders: [David]
---

# The core is built, not adopted

Answers the open decision recorded in ADR-0013's "More information". Builds on ADR-0007
and ADR-0010, both of which remain in force.

## Context and problem statement

ADR-0013 found that a Rust SysML v2 and KerML library already exists: `syster-base`
(MIT, crates.io, `0.4.0-alpha` when ADR-0013 was written), with a lossless parser over
`logos` and `rowan`, Salsa-based incremental semantic analysis, scope-aware name
resolution, IDE features, and a bundled standard library. It also noted an independent
Rust parser with an extensive snapshot corpus. It asked that build versus adopt be
raised as this record "before implementation starts", because adopting a core changes
the cost of everything above it.

That did not happen in that order. The parser was built in the interval, and at the time
of writing reads 166 of 558 productions against a reference grammar derived in full and
frozen. So this record is partly a capture: it states the decision the build has already
made, and it argues that decision from the drivers rather than from the code that now
exists. The code already written is not one of the reasons. A decision justified by its
own sunk cost is not a decision.

ADR-0007 already settled that the resolver runs in process and that the tool is
authoritative about its own semantics. It did not settle whose code that resolver is.
Adopting a library would satisfy "local" and still fail "owned", so the question is
narrower than it looks, but it is not closed by ADR-0007 alone.

## Decision drivers

- **DD-1. Invariant 4.** Every claim about the language traces to a clause citation, a
  corpus file, or a recorded deviation. The derivation pipeline (ADR-0010, ADR-0011,
  ADR-0014, ADR-0015), the frozen reference grammar, and the coverage markers exist to
  make that checkable by the gate. A claim made inside an adopted crate is asserted by
  its maintainers and checked by their tests, whatever the quality of either.
- **DD-2. Semantic authority (ADR-0007).** The tool must be the author of its own
  conformance interpretation, because DEAL normalizes against it. Borrowing an
  interpretation over an API and borrowing one as a crate differ in latency, not in
  whose interpretation it is.
- **DD-3. The delivery estate.** This repository feeds an air-gapped, ATO-governed
  estate. Every dependency is pinned, license-checked and reviewed through `cargo deny`,
  and a dependency's upgrades are reviewed as changes to what is delivered. A pre-1.0
  crate at the base of the workspace makes every upstream release such a review.
- **DD-4. Single-maintainer sustainment.** This one cuts both ways. Building costs time
  this project has little of. Adopting a pre-1.0 core with a small maintainer base
  imports its breaking changes and its bus factor, and a core cannot be vendored and
  frozen for long without becoming a fork.
- **DD-5. The workspace invariants are enforced here.** Losslessness (invariant 1), the
  IR admitting what parses (invariant 2), and no panic on any input (invariant 3) are
  held by this workspace's lint table, property tests and corpus sweep. Clippy's
  `unwrap_used`, `indexing_slicing` and `string_slice` lints apply only to workspace
  code, so they would not reach an adopted core.

## Considered options

1. **Adopt `syster-base` as the core.** Its parser, semantic analysis and standard
   library become the base of the workspace; sv2 builds views, the studio and extensions
   above it.
2. **Build the syntax layer, adopt the semantic layer.** Keep `sv2-syntax` and
   `sv2-ast`, and bridge their tree into `syster-base`'s name resolution and analysis.
3. **Build the core.** `sv2-syntax` through `sv2-resolve` are this workspace's own code.
   The Pilot Implementation and `syster-base` are references, not dependencies.

Sourcing semantics from the Pilot over the Systems Modeling API was rejected by ADR-0007
and is not reopened here.

## Decision outcome

**Option 3.** The core is built. Nothing in `sv2-syntax`, `sv2-ast`, `sv2-hir` or
`sv2-resolve` depends on another implementation of the language.

The decision holds whatever the current state of `syster-base` is, which is why it can
be written without evaluating that library first. DD-1, DD-2 and DD-5 are about who owns
the claims and who enforces the invariants, not about how good the other
implementation is. A better `syster-base` would make Option 1 cheaper; it would not make
its claims traceable through this gate.

**What the other implementations are for.** They stay useful, in ways that leave
authority here:

| Implementation | Role | Bound |
|---|---|---|
| Pilot Xtext | Tier B input: production inventory, token set, production-to-metaclass map | ADR-0010: never rule bodies |
| `syster-base` | Design reference; optional behavioral oracle | Not in `Cargo.lock`. Never cited as evidence under invariant 4 |
| Independent Rust parser | Design reference | Not in `Cargo.lock` |

Used as an oracle, `syster-base` runs outside the workspace, the way `_earley.py` is an
oracle for the grammar: a second opinion that shares no code with the thing it checks.
A disagreement becomes a finding to adjudicate against the specification and is
recorded in `.claude/state/deviations.json`. It is never resolved by deferring to the
other implementation. Code copied from an MIT-licensed reference is permitted,
attributed in `NOTICE.md`, and still owes the clause citation invariant 4 requires of
any code written here.

### Consequences

Good: every claim the parser and resolver make stays checkable by the gate, and the
invariants stay enforced by lints and tests this workspace runs. The dependency closure
of the core stays at what it is today, which is small enough for a reviewer to read.
No upstream release cadence sets the pace of this project.

Bad: the resolver is the largest line item in the project (ADR-0007), and all of it is
owned here: library loading, visibility, name resolution, derived properties and
constraint checking. IDE features that an adopted core would bring with it, semantic
tokens among them, have to be written. And the incremental query engine that
`syster-base` would have supplied has to be chosen or built, which is its own decision.

Neutral: the Pilot remains the practical reference for how the language behaves on
real models. This record changes nothing about how it is used, only states that
`syster-base` joins it on the same terms.

## Pros and cons of the options

**Option 1, adopt `syster-base`.** Good, because it is the least code: a parser,
resolver, standard library and IDE features exist today, and Salsa-based incrementality
comes with them. Bad, because every claim about the language becomes its maintainers'
(DD-1, DD-2). Bad, because invariants 1 through 3 would be properties this workspace
hopes for rather than enforces (DD-5). Bad, because a pre-1.0 core with a small
maintainer base is the most exposed position a delivered dependency can hold (DD-3,
DD-4).

**Option 2, build syntax, adopt semantics.** Good, because the traceable part is kept
and the expensive part is borrowed. Bad, because the resolver is where semantic
authority lives, so DD-2 fails exactly where it matters most. Bad, because two tree
representations need a bridge that must be kept in step with two independent release
histories, and ADR-0004's lossless tree would be translated into a structure this
workspace does not control. It costs most of Option 3's discipline and most of
Option 1's exposure.

**Option 3, build the core.** The chosen option. It costs the resolver and the query
engine, and it keeps the gate the authority on everything the tool says about the
language.

## More information

### What this record does not decide

**The incremental query engine.** Salsa, a hand-rolled query layer, or plain recomputation
are all open. Choosing now would be a guess: `state.json` still carries `crate-split` as
an open question about the boundary between `sv2-hir` and `sv2-resolve`, pending implied
specialization injection, and the query engine depends on the same unknown — what the
units of invalidation are, and how far a change propagates. It gets its own record,
decided before `sv2-resolve` publishes its first public query. The criteria can be
stated now:

- re-resolution after a one-file edit fits ADR-0013's FIT-4 budget on the largest
  standard library file;
- no network access at build, test or run time;
- it gives ADR-0019 R-2's "cache outside the query system" a precise meaning;
- it runs where the resolver runs and not in `sv2-wasm` (ADR-0018), so WebAssembly
  support is not a requirement;
- it passes `cargo deny check bans licenses sources` without an exception.

A spike against real name resolution, once it exists, settles it.

**How good `syster-base` is.** Not evaluated, and not needed for this decision. It
would be needed before using it as an oracle, and that use is optional.

### Review triggers

- OMG or the Systems-Modeling organization publishes a normative machine-readable
  concrete syntax, or sponsors a Rust implementation it undertakes to sustain. That
  changes DD-1 and DD-4.
- The project gains more maintainers or program funding, which changes the cost side of
  DD-4.
- ADR-0007 is superseded.

### Related decisions

- [ADR-0007](0007-local-resolver.md) — the resolver is local and owned; this record
  settles whose code it is.
- [ADR-0010](0010-grammar-and-metamodel-sourcing.md) — the Pilot is Tier B and never a
  source of rule bodies; `syster-base` joins it on the same terms.
- [ADR-0011](0011-specification-bnf-as-a-pinned-input.md) — the specification BNF as a
  pinned input, which is what the built parser is derived from.
- [ADR-0004](0004-lossless-syntax-tree.md) — the lossless tree an adopted semantic layer
  would have had to translate.
- [ADR-0013](0013-rust-cst-via-webassembly-as-code-mirror-syntax-tree-source.md) — where
  this decision was first named as open.
- [ADR-0019](0019-extensions-are-in-tree-consumers-of-the-resolved-model.md) — the query
  surface extensions consume, whose caching rule the deferred query-engine record must
  make precise.