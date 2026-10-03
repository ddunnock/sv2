---
name: optimize-parser
description: Make the parser faster as a measured optimization series — profile, change one thing, prove each step with scripts/perf.sh, close on a net win. Use when the user asks to optimize, speed up, or reduce allocations or lookahead in sv2-syntax, or names an item from docs/perf/assessment.md.
---

# Optimize the parser

ADR-0024 is the contract. The short version: every step is measured, every step improves
its primary metric or says what later step it enables, nothing gets worse past noise,
nothing the parser produces changes at all, and the series ends on a net win or is
abandoned and reverted.

## 1. Pick one target, and its metric

One hypothesis per series. Start from `docs/perf/assessment.md` if the user did not name
one. Choose the primary metric the change should move:

| Primary | When |
|---|---|
| `peeked` | lookahead work: recognisers, rescans, the `at_*` dispatch |
| `allocations` / `allocated_bytes` | builder, `Vec`, `String`, diagnostic churn |
| `time` | CPU-bound work no counter sees (branching, cache layout); noisier, so needs 3% |

## 2. Measure and profile first

```bash
scripts/perf.sh measure
cargo bench -p sv2-syntax --bench parse
cargo flamegraph -p sv2-syntax --bench parse -- --bench simple_vehicle_model
```

Write down what the profile says before changing anything. A change made without a
profile is a guess, and guesses are what enabling steps get abused for.

## 3. Open the series

The working tree must be clean in `crates/sv2-syntax/src`.

```bash
scripts/perf.sh series open <short-name> --primary <metric>
git add .claude/state/perf-series.json && git commit -m "Open optimization series <name>"
```

From here until the close, the gate fails whenever the parser source differs from the last
measured step. The Stop hook runs the gate, so a turn cannot end over an unmeasured change.

## 4. One change, then prove it

For each step:

1. Make one change.
2. `./scripts/gate.sh`, which must be green. The tests, roundtrip, corpus sweep and snapshots
   all hold exactly as before. **A snapshot that moved means the change is wrong**, not
   that the snapshot needs review: an optimization does not change a tree.
3. `scripts/perf.sh series step`. It also checks the output fingerprint, so a changed tree,
   error, or deviation note anywhere in the corpus rejects the step however fast it is.
4. **Improved:** commit the code with the updated ledger, one commit.
5. **Rejected:** nothing was recorded. Rework the change or `git checkout` it. Do not
   re-run hoping the timing noise lands differently more than once; a step that passes
   only on a retry did not improve.

### When `--enabling` is honest

`scripts/perf.sh series step --enabling "<what later step this makes possible>"`

Use it when the change is a precondition for the win, not a win: introducing an index
before the lookups that will use it, changing a representation before the code that
exploits it. The reason names the step it enables. At most three in a row; the fourth
step must show the improvement or the series is not working.

Do not use it for a change that was meant to improve and did not. That is a rejected step.

## 5. Close or abandon

```bash
scripts/perf.sh series close
```

Closing needs a net win over the series' opening on the primary metric, with nothing else
worse. It re-records `tests/perf-baseline.json`, so the gate's ratchet tightens to the
new level. Commit the ledger and baseline together.

If the series cannot get there:

```bash
scripts/perf.sh series abandon --reason "<what was learned>"
git revert <opening>..HEAD   # the range it prints
```

An abandoned series with a good reason is a finding. Record what was learned in
`docs/perf/assessment.md` against the item, so nobody tries it again blind.

## Never

- Edit `tests/perf-baseline.json` or `.claude/state/perf-series.json` by hand. The hook
  blocks it, and the reason is the same as for every derived file here.
- Change a tolerance in `scripts/perf.py` to get a step through. Tolerances are
  ADR-0024's; changing one is a new decision, argued there.
- `perf.sh record` during a series (it refuses), or to absorb a regression an
  optimization caused. `record` is for grammar work that legitimately costs more.
- Trade the lossless invariant (ADR-0004) or error recovery for speed. Faster and wrong is
  wrong.
