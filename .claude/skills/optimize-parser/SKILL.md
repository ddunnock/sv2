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

### Deeper, per platform

These are for diagnosis only. Nothing here feeds a verdict, and the gate must give the
same answer on both machines.

**macOS: Instruments.** `cargo flamegraph` already records through `xctrace`, but the
Instruments app shows more. Build the bench with symbols, then record one of its cases:

```bash
CARGO_PROFILE_BENCH_DEBUG=true cargo bench -p sv2-syntax --bench parse --no-run  # prints the binary
xcrun xctrace record --template 'Time Profiler' --output /tmp/parse.trace \
  --launch -- target/release/deps/parse-<hash> --bench simple_vehicle_model
open /tmp/parse.trace
```

| Template | Use it for |
|---|---|
| Time Profiler | Invert the call tree to walk from a hot leaf (`memcmp`) back to its callers; time per source line |
| Allocations | *Where* allocations happen, for an `allocations` / `allocated_bytes` series; the probe only says how many |
| CPU Counters | Instructions, cycles, branch mispredictions: *why* a hot path is slow. Steadier than time, but not a gate |
| Processor Trace | Instruction-level trace; needs an M4-generation chip or later |

**RHEL 9: perf and callgrind.** `cargo flamegraph` uses `perf` there. For counts close to
exact on one function, run `valgrind --tool=callgrind` on the bench binary and read it with
`callgrind_annotate`. Instruction counts from Linux are not comparable with the Mac's.

### Under WebAssembly (FIT-4)

ADR-0013 FIT-4 is about the editor's wasm build, not native. Check it at the start and
the close of a series:

```bash
scripts/perf.sh wasm
```

It times the five largest files natively, under wasmtime, and under Bun. Bun's
JavaScriptCore is the engine of Tauri's webview on macOS and Linux, so it is the closer
stand-in. The command reports each engine's ratio to native and the largest file's p95
against the 16 ms budget. The guest sees only a temporary copy of the files, and an
engine that is not installed is skipped. It is reported, never gated.

**One-time setup, on each machine** (a setup download; nothing fetches at build or gate
time):
- **wasm target:** installed with the pinned toolchain from `rust-toolchain.toml`. If an
  older checkout predates that line, run `rustup target add wasm32-wasip1` in the repo.
- **wasmtime:** `curl https://wasmtime.dev/install.sh -sSf | bash`. It installs to
  `~/.wasmtime/bin`, which `perf.sh` finds even when it is not on `PATH`.
- **Bun:** `curl -fsSL https://bun.sh/install | bash`. It installs to `~/.bun/bin`; on
  RHEL 9 this works from the home directory, with no root needed.

### Which machine

Counters and the output fingerprint are the same on both machines, so a series can move
between them. **Time is not.** Each measurement records its host. A step taken on another
machine is judged on counters alone, and its verdict says the time was not compared.

A series opened with `--primary time` must step and close on the machine it was opened on.
Open it where you will finish it.

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
2. `scripts/perf.sh series step`. It measures the change and also checks the output
   fingerprint, so a changed tree, error, or deviation note anywhere in the corpus rejects
   the step however fast it is. It comes before the gate, because inside a series the gate's
   ratchet fails until the change is measured.
3. `./scripts/gate.sh`, which must be green. The tests, roundtrip, corpus sweep and snapshots
   all hold exactly as before. **A snapshot that moved means the change is wrong**, not
   that the snapshot needs review: an optimization does not change a tree. Adding tests
   can make `state.json`'s generated block stale; regenerate it with
   `python3.12 .claude/scripts/regen_state.py`.
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
