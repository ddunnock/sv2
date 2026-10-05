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
| p95, largest file, WebAssembly | 11.3 ms JavaScriptCore (Bun), 12.5 ms wasmtime | `scripts/perf.sh wasm` |
| WebAssembly / native | 1.21–1.29× JavaScriptCore, 1.37–1.43× wasmtime | `scripts/perf.sh wasm`, five largest files |
| Scaling ×1 → ×16 copies | 8.46 → 8.94 MB/s: linear | bench `repeated` |
| Tokens peeked per token consumed | 219 (largest file), 340 (corpus) | counters |
| Allocations, largest file | 9,891 (1.31 MB) | counters |
| Corpus totals (311 files) | 132,472 allocations, 23,986,614 peeked, 70,576 consumed | `tests/perf-baseline.json` |

**FIT-4 is met, with modest headroom.** This document first estimated WebAssembly at
1.5–2× native, which would have put the largest file at 15–20 ms, at or past the budget.
Roadmap Phase 1 measured it instead: 1.2–1.4×, so 11.3–12.5 ms, inside the 16 ms budget
by 3.5–4.7 ms. That headroom is on a 73 KB file. At the measured throughput the budget
fits about 100 KB, and the standard library's largest file is not yet measured.

Lookahead is a constant factor, not a growth rate: scaling is linear, so the parser is
slow, not quadratic.

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
| 3.4% + 2.3% | `at_name`, `nth_is_name` | `is_name` scans all 173 `KEYWORDS` linearly |
| ~8% | rowan `NodeCache` and green nodes | building the tree: the irreducible part |
| 2.6% | `tokenize` | the lexer |

**About 70% of parse time is lookahead re-deciding what is already known.** It has three
causes:

1. **Keyword identity is recomputed by string comparison, every time it is asked.** The
   lexer emits keywords as `BasicName`. `nth_is_keyword(n, "part")` compares text, and
   `is_name` calls `keyword()`, a linear scan of 173 entries with a string compare each,
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
most of that time belongs to other callers. The exact counts below replace those edges.

Grouping self time by its nearest `at_*` ancestor: **65.1% of parse time runs under a
recogniser.** The largest are `at_simple_usage` (8.5%), `at_sysml_keyword_member` (7.6%),
`at_simple_definition` (5.7%), and a long tail of about 1% each across the 28 alternatives.

## Exact counts

Roadmap Phase 0 (9768f88): counters at the hot helpers, deterministic and the same on
every platform. From `scripts/perf.sh measure`; the reference copy is in
`tests/perf-baseline.json` under each file's `detail`.

| Counter | Largest file | Per token | Per decision | Corpus | Per token | Per decision |
|---|---:|---:|---:|---:|---:|---:|
| `keyword_lookups` | 25,091 | 3.6 | 23.7 | 195,225 | 2.8 | 17.7 |
| `keyword_entries` | 4,126,749 → 225,819 after Phase 2 | 592.0 → 32.4 | 3,900.5 → 213.4 | 31,520,868 → 1,757,025 | 446.6 → 24.9 | 2,863.5 → 159.6 |
| `is_name` | 24,477 | 3.5 | 23.1 | 201,923 | 2.9 | 18.3 |
| `nth_is_keyword` | 1,233,818 | 177.0 | 1,166.2 | 20,355,352 | 288.4 | 1,849.1 |
| `qualified_names` | 17,763 | 2.5 | 16.8 | 119,868 | 1.7 | 10.9 |
| `skip_occurrence_usage_prefix` | 57,320 | 8.2 | 54.2 | 972,243 | 13.8 | 88.3 |
| `skip_basic_usage_prefix` | 70,192 | 10.1 | 66.3 | 1,210,455 | 17.2 | 110.0 |
| `skip_prefix_metadata` | 138,283 | 19.8 | 130.7 | 2,228,462 | 31.6 | 202.4 |
| `member_decisions` | 1,058 | 0.2 | 1.0 | 11,008 | 0.2 | 1.0 |
| `member_dispatch` | 877 | 0.1 | 0.8 | 8,257 | 0.1 | 0.8 |
| `keyword_member_dispatch` | 2,153 | 0.3 | 2.0 | 36,360 | 0.5 | 3.3 |

"Per token" is per consumed token: 6,966 on the largest file, 70,576 over the corpus.
"Per decision" is per member decision. Tokens peeked per decision: 1,444 on the largest
file, 2,179 over the corpus.

**What the counts say, and where they correct the profile:**

1. **`nth_is_keyword` is the dominant lookahead operation.** It runs 1.23 M times for 6,966
   consumed tokens, 1,166 times per member decided, and each call is a string compare.
   Phase 2 makes each call cheap; Phase 5 makes there be fewer.
2. **`keyword()` compared 164 table entries per lookup**: 4.1 M string compares on one
   file. That was the linear scan, exactly. Phase 2's step 1 made it a binary search,
   at 9 probes per lookup: the standard library's bound for 173 entries. It is not 8,
   because that search never stops early.
3. **Prefix skipping repeats** 54 to 202 times per member decision, for prefixes that are
   almost always empty. `skip_prefix_metadata` alone runs 131 times per member on a file
   with almost no `#` metadata. This is Phase 4's whole target.
4. **Qualified names are *not* hot**: 2.5 measurements per consumed token. The sampled
   12.3% self time for `qualified_name_length` was inlining smear. The counts win, so
   Phase 4 no longer includes them.
5. **The keyword-member chain is re-asked** 2.0 times per decision on the largest file and
   3.3 over the corpus: once to decide that a member starts, and again to decide which.

### Prefix-skip start positions (Phase 4, step 0)

A throwaway log recorded every prefix-skip call's absolute meaningful position and
comment mode over the 311-file corpus, then replayed them against caches of 1–32 slots.
The log was reverted and is not in the parser. The table gives misses as a share of
calls. "Distinct" is the floor: each start computed exactly once.

| Helper | Calls | Distinct | 1 slot | 4 slots | 8 slots, direct-mapped |
|---|---|---|---|---|---|
| `skip_prefix_metadata` | 2,228,462 | 12,029 (0.54%) | 2.22% | 1.18% | 0.54% |
| `skip_occurrence_usage_prefix` | 972,243 | 7,906 (0.81%) | 4.30% | 2.36% | 0.81% |
| `skip_basic_usage_prefix` | 1,210,455 | 7,907 (0.65%) | 3.09% | 1.81% | 0.65% |

- **Each helper starts at about one distinct position per member decision:** 0.7–1.1
  over the corpus's 11,008 decisions, and 0.8–1.0 on the largest file. The 54–202 calls
  per decision are one computation asked for again and again.
- **8 direct-mapped slots, indexed by `position % 8`, reach the floor exactly** on the
  corpus and on the largest file. So do 16 and 32. LRU replacement does no better, so a
  plain mask is enough.
- **Even one slot hits 96–98%.** The repetition is almost entirely back-to-back calls at
  one start.
- **The replay counts every call in the uncached stream.** Once the outer
  `skip_occurrence_usage_prefix` hits, the inner `skip_basic_usage_prefix` and
  `skip_prefix_metadata` calls it would have made never happen. The real counts can
  only be lower.

Phase 4 therefore uses an 8-slot `[Cell<Entry>; 8]` per helper, keyed by
`(position, comments_significant)`.
   Phase 5's head computation answers both at once.

### Keyword-member dispatch by head (Phase 5, step 0)

Measured at 723eb60, after Phase 4. A throwaway worktree wrapped each of
`at_sysml_keyword_member`'s 28 recognisers and recorded, per call, the member's **head**
and what each recogniser peeked and answered. The head is the first token after any
`#` prefix metadata and the prefix words `in out inout derived abstract variation constant
ref individual snapshot timeslice`, as a keyword or a token kind. The peek counter was
restored around the head computation, so the totals are the parser's own. The worktree
was deleted; nothing from it is in the parser.

| Where | Calls | Peeked | Share of 4.36 M |
|---|---:|---:|---:|
| `at_sysml_keyword_member`, all callers | 14,020 | 2,395,083 | 55% |
| `at_member_element` (contains most of the above) | 8,257 | 1,486,292 | 34% |
| `at_result_expression` | 688 | 140,403 | 3% |
| `at_source_succession_member` | 5,681 | 12,998 | 0.3% |

Of the chain's 2.40 M:

| Recogniser | Calls | Accepts | Peeked |
|---|---:|---:|---:|
| `at_definition_element` (asked first, every time) | 14,020 | 1,961 | 1,138,651 |
| `at_simple_usage` | 10,378 | 3,004 | 245,453 |
| `at_action_usage` | 12,059 | 283 | 165,621 |
| `at_individual_or_portion_usage` | 11,012 | 95 | 141,492 |
| `at_reference_usage` | 7,374 | 117 | 122,840 |
| `at_succession_as_usage` | 10,917 | 86 | 113,730 |
| `at_binding_connector_as_usage` | 10,831 | 71 | 112,522 |
| the other 21 | | | 354,654 |

- **68% of the chain's peeks are spent in recognisers that never accept at that head**
  anywhere in the corpus: 1,622,532 of 2,395,083. That share is what head dispatch can
  remove, and it is 37% of all peeks. It is an upper bound from one corpus. The candidate
  sets themselves must come from the recognisers' code, not from this log.
- **`at_definition_element` alone spends 728,419 peeks at heads where it never wins.** At
  a bare-name head it costs 89 peeks per call and wins none of 3,670 calls.
- **An `end` member costs about 3,600 peeks per call:** 150 calls, 538,556 peeks.
  `skip_end_usage_prefix` walks the cross feature token by token, and at each token
  `at_end_kind` runs the whole 28-recogniser chain. Head dispatch makes each nested call
  cheap. Memoizing `skip_end_usage_prefix` stops the walk being repeated.
- **70 distinct heads; 31 have more than one winner.** These account for 11,009 of the
  14,020 calls. `part`, for example, is `at_definition_element` (`part def`) or
  `at_simple_usage`. So the head selects an ordered candidate set, not one recogniser.
- **1,787 calls end with no winner** (bare names, `:`, `[`, numbers, `then`). They cost
  290,998 peeks. Most come from `at_result_expression` and `at_end_kind` asking whether a
  member starts at a token that cannot start one.
- **Correction to the Phase 0 count:** `keyword_member_dispatch` is now 14,020, 1.3 per
  decision, not 36,360 (3.3). Phase 4 removed the repeats: the nested calls came from
  `at_end_kind` inside prefix skips that are now cached.
- **`membership` decides a second time.** After `at_member_element` accepts,
  `definition_element` and `usage_element_of_class` re-run their own recogniser chains
  to choose which production to read. That cost is inside the 1.8 M peeks that fall
  outside the dispatch regions above.

### Keyword classification by token (2b), abandoned

Series `token-keywords` (2026-10-05, opened at efe9024, RHEL 9 workspace) tested whether
the roughly 27% of self time spent asking which keyword a token is (`nth_is_keyword`,
`is_reserved`, `admits`, and their `memcmp`) could be removed by classifying each token
once in the lexer. It could not, because the text comparison was not where that time
goes.

Instructions, by `callgrind` over the five largest files (`perf_probe timing 1`), which
is deterministic where this workspace's timing is not:

| Build | Total | `nth_is_keyword` | `is_reserved` | `memcmp` | `tokenize` |
|---|---:|---:|---:|---:|---:|
| before | 412.2 M | 96.7 M | 8.1 M | 19.6 M | 27.1 M |
| step 1: `Token::keyword` set in `tokenize`, `is_reserved` by kind | 407.4 M (−1.2%) | 96.7 M | — | 17.1 M | 34.8 M |
| step 2: `nth_is_keyword` answers a non-keyword name without a compare | 406.3 M (−1.4%) | 96.0 M | — | 16.9 M | 34.8 M |

- **`nth_is_keyword` costs about 50 instructions a call** over 1.9 M calls. Almost all of
  it is the lookahead path the compare sits behind: `peek_nth`, `cursor_position`, the
  meaningful-token index, and the counter hook. The compare itself is short, and most
  calls land on tokens that are keywords, so an early exit for names rarely fires.
- **Removing `is_reserved`'s search** (8.1 M) was mostly repaid in `tokenize` (+7.7 M),
  which classifies every name, including names nothing asks about.
- **The timed verdict misled.** The series ledger recorded step 1 at −9.9% time. A
  pinned A/B over five alternating rounds gave −10.6, −9.1, +5.1, −2.0 and −1.5%, so
  about −2% at the median. This workspace's GPU-less desktop keeps three to four of its
  eight cores busy. Even pinned with `taskset`, the median spreads by about 5%, which is
  wider than a `time` series' 3% bar. A `time` series here needs an instruction count
  beside it, or the MacBook.
- **Not to retry:** per-token keyword classification as a speed change. It may still be
  worth having for clarity: `Token::keyword` made `is_name` a kind test. The next lever on
  this cost is the number of `nth_is_keyword` calls, or the per-call cost of `peek_nth`,
  not the comparison.

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
