# Parser performance assessment

Where the parser spends its time, and the candidate optimizations ranked by expected gain
against risk. Each candidate is meant to run as its own series under the
`optimize-parser` skill (ADR-0024). Nothing here has been changed yet.

Measured 2026-10-03 at 7020f37, arm64 macOS, release build.

## Where it stands

| Measure | Value | Source |
|---|---|---|
| Throughput, largest file (73.5 KB) | ~8.5 MB/s, median 8.7 ms | `cargo bench --bench parse` |
| p95, largest file | 9.9 ms native, against FIT-4's 16 ms | `scripts/perf.sh measure` |
| Scaling ×1 → ×16 copies | 8.46 → 8.94 MB/s: linear | bench `repeated` |
| Tokens peeked per token consumed | 219 (largest file), 340 (corpus) | counters |
| Allocations, largest file | 9,891 (1.31 MB) | counters |
| Corpus totals (311 files) | 132,472 allocations, 23,986,614 peeked, 70,576 consumed | `tests/perf-baseline.json` |

**FIT-4 has thin headroom.** 9.9 ms is native. FIT-4 names WebAssembly in the editor, which
typically runs 1.5–2× slower than native: 15–20 ms, at or past the 16 ms budget, and on a
73 KB file rather than the standard library's largest. Lookahead is a constant factor,
not a growth rate: scaling is linear, so the parser is slow, not quadratic.

## Where the time goes

Self time, `cargo flamegraph --bench parse -- --bench simple_vehicle_model`, 996 samples:

| Self | Function | What it does |
|---|---|---|
| 17.9% | `nth_is_keyword` | compares a token's text to a keyword string |
| 14.1% | `memcmp` (+ its dyld stub) | the string comparisons underneath it and `is_name` |
| 13.7% | `skip_basic_usage_prefix` | up to 9 keyword compares, re-run by every recogniser |
| 12.3% | `qualified_name_length` | re-measures the same qualified name per recogniser |
| 4.0% | `skip_occurrence_usage_prefix` | the same, for occurrence prefixes |
| 3.7% | `skip_one_prefix_metadata` | the same, for `#Metadata` prefixes |
| 3.4% + 2.3% | `at_name`, `nth_is_name` | `is_name` scans all 217 `KEYWORDS` linearly |
| ~8% | rowan `NodeCache` and green nodes | building the tree: the irreducible part |
| 2.6% | `tokenize` | the lexer |

**About 70% of parse time is lookahead re-deciding what is already known.** It has three
causes:

1. **Keyword identity is recomputed by string comparison, every time it is asked.** The
   lexer emits keywords as `BasicName`. `nth_is_keyword(n, "part")` compares text, and
   `is_name` calls `keyword()`, a linear scan of 217 entries with a string compare each,
   for every candidate name.
2. **Member dispatch is a chain of 28 recognisers** (`at_sysml_keyword_member`, and its
   siblings in `body_element`). Each re-skips the same usage prefix from the same
   position before looking at the one keyword that decides it.
3. **Qualified names are re-measured** by each recogniser that looks past one.

### A caveat on caller attribution

The self times above are sampled from a release build, where inlining merges callers into
their callees. Self time per function is reliable. **Caller edges are approximate.** The
profile credits `qualified_name_length` called from `skip_one_prefix_metadata` with 10.7%,
but that function returns at once unless the token is `#` (`parser/metadata.rs:38`), so
most of that time belongs to other callers. Exact call counts per helper are roadmap
Phase 0.

Grouping self time by its nearest `at_*` ancestor: **65.1% of parse time runs under a
recogniser.** The largest are `at_simple_usage` (8.5%), `at_sysml_keyword_member` (7.6%),
`at_simple_definition` (5.7%), and a long tail of about 1% each across the 28 alternatives.

## What the processor is doing

Instruments CPU Counters, bottleneck mode, on the same bench (88 10-ms buckets while
parsing; the timer-calibration buckets are excluded):

| Share of cycles | Category | Reading |
|---|---|---|
| 66.5% | useful | the processor is mostly not stalled: the work is real, just redundant |
| 19.2% | instruction delivery (front end) | control flow hopping across many small recognisers |
| 8.4% | discarded (speculation) | branch mispredictions: present, not dominant |
| 5.9% | processing | arithmetic and memory: not the bottleneck |

The parser is slow because it does too much, not because what it does is
cache-unfriendly. The remedy is less work: Phases 2, 4 and 5. Phase 5's dispatch should
also cut the front-end share.

## Where the allocations come from

Instruments Allocations, five parses of the largest file plus divan's setup: 50,747
allocations, matching the probe's 9,891 per parse.

| Count share | Size class |
|---|---|
| 47.2% | 48 bytes |
| 22.0% | 64 bytes |
| 8.8% | 80 bytes |
| 5.4% + 4.9% | 112 and 96 bytes |

69% of allocations are 48 or 64 bytes. These are, by size, rowan green nodes with one or
two children; the export gives size classes, not backtraces, so this is an inference.
Those nodes *are* the tree, and reducing them would change its shape, which an
optimization may not do. What can be trimmed is the buffers (roadmap Phase 6).

## The plan

The optimizations are phased in **[roadmap.md](roadmap.md)**. Each phase can be planned
and executed alone, as one `optimize-parser` series.

One ordering changed from this document's first draft. Memoizing prefix and name scans
(roadmap Phase 4) now comes *before* member-head dispatch (Phase 5), not as its fallback.
It makes the head cheap to compute, and it is the lower-risk win if Phase 5 has to be
abandoned.
