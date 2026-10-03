---
title: "Parser performance is measured by deterministic counters, ratcheted, and optimized in series"
status: proposed
date: 2026-10-03
deciders: [David]
---

# Parser performance is measured by deterministic counters, ratcheted, and optimized in series

## Context and problem statement

The parser had no measurement. Its only performance claim is ADR-0013 FIT-4: p95
highlight latency under 16 ms on the largest standard library file. It has already had one
accidental quadratic (lookahead by index, fixed in 6f85693), found by reading rather than
by any check, and nothing would catch the next. Optimization work, when it comes, needs a
rule for what counts as an improvement — including for a change that only pays off once
later changes land.

Wall-clock time is what users feel, but it depends on the machine and what else it is
doing, so a gate that reads it flips on a busy laptop. A gate that flips gets ignored.

## Decision drivers

- A gate must be deterministic: the same tree gives the same verdict, on any machine.
- What the editor feels is time, so time must still have a say.
- Quadratic behaviour is the regression that matters most, and the one a fixed tolerance
  on one file catches least.
- Grammar work legitimately adds cost; the ratchet must not fire on every production.
- Optimization needs room for enabling changes, without room for drift.
- `unsafe_code` is forbidden workspace-wide, so measuring cannot need `unsafe` here.

## Considered options

- **Wall-clock only**, against a saved baseline with a noise threshold. Measures the right
  thing; cannot gate reliably.
- **Deterministic counters only.** Reproducible; blind to CPU-bound wins (branching,
  cache use) that change no count.
- **Both, counters decide** — chosen.
- Instruction counts (callgrind) were not available: no valgrind on the arm64 macOS the
  project is developed on.

## Decision outcome

**Measure two ways, from one probe** (`crates/sv2-syntax/examples/perf_probe.rs`):

- *Counters*, deterministic: allocations and bytes allocated (`stats_alloc`, which keeps
  the `unsafe` of a counting allocator out of this workspace), and tokens peeked and
  consumed, counted by two relaxed atomics behind the `counters` feature, which compile to
  nothing without it. And a *scaling ratio*: tokens peeked per byte for the largest file
  repeated eight times in one input, against one copy. Linear work gives 1.0; a rescan
  from the start gives about the number of copies.
- *Timing*, from a release build without the feature: median and p95 over 30 runs after
  three warm-up runs, for the five largest corpus files.

**The ratchet, for all work** (`scripts/perf.sh check`, in the gate): fails when a
counter's corpus total grows more than 10% over `tests/perf-baseline.json`, when one
file's grows more than 50%, or when the scaling ratio exceeds 1.25. It reads no timing.
Cost a change legitimately adds is re-baselined with `perf.sh record --reason`, a
recorded act, like the corpus ledger's `--record`.

**Optimization series** (`perf.sh series ...`, ledger `.claude/state/perf-series.json`):

| Rule | Value | Why |
|---|---|---|
| A step improves its primary metric | ≥1% (counter), ≥3% (median time) | more than rounding; more than timing noise |
| …or is declared enabling | `--enabling "<reason>"` | the flex: a change that pays off with later ones |
| Enabling steps in a row | at most 3 | flex without indefinite drift |
| No step worsens a counter | >2% rejects | counters do not wobble, so a small allowance suffices |
| No step slows median time | >5% rejects | time has a veto, never a vote |
| A series closes | on a net win over its opening, nothing else worse | the enabling steps have to have paid off |
| Closing | re-records the baseline | the ratchet tightens to where the work ended |
| While open | the gate fails if the parser source differs from the last step's | no change goes unmeasured |

A rejected step is not recorded; the change is reworked or reverted. A series with no
win is abandoned with a reason, and its commits reverted.

FIT-4's file is the standard library's largest, which is not vendored. Until it is, the
largest corpus file stands in, measured natively; the WebAssembly figure FIT-4 actually
names is not measured here.

### Consequences

- Good, because the gate's verdict is reproducible, and the quadratic class is caught by
  construction rather than by a tolerance happening to be tight enough.
- Good, because optimization has a rule for "better" that admits enabling work and still
  ends in a measured win.
- Good, because the shipped parser carries no counting cost.
- Bad, because counters miss CPU-bound wins; a series with primary `time` exists for those,
  and pays for it in timing noise.
- Bad, because the baseline needs re-recording after grammar work that adds real cost, and
  a reason written each time.
- Bad, because two dev-dependencies (`divan`, `stats_alloc`) join the tree.

### Confirmation

`scripts/tests/test_perf.py` covers every verdict, each positive case with its negative.
The negative controls run when this was built: an allocation planted in `peek_nth` failed
the totals; a rescan planted in `bump_as` failed scaling at 7.68; an open series failed the
gate over an unmeasured change, rejected a non-improving step, accepted a declared enabling
one, and refused to close. That control series is in the ledger, abandoned.

Changing a tolerance is changing this decision: it is done here, with its reason, never in
`perf.py` alone to clear a gate.
