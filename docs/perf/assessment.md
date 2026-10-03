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

## Candidates, ranked

### 1. Classify keywords once, at the token: est. 25–35% of time

Compute each `BasicName` token's keyword kind once in `Parser::new`, beside the
meaningful-token index. Then:
- `is_name` becomes a lookup, not a 217-entry scan;
- `nth_is_keyword` compares a `SyntaxKind`, not text;
- `keyword()` becomes a `match` (generated) or a binary search over the sorted table.

- **Primary metric:** `time`. No counter sees string compares; `peeked` will not move.
- **Steps:**
  1. Precompute the table, and make `is_name` use it. This step should already win.
  2. Make `nth_is_keyword` compare kinds; its `&str` callers keep working through a
     kind lookup.
  3. Optionally move call sites from `"part"` to `SyntaxKind::KwPart`. This is mechanical
     but touches hundreds of lines, so do it one module per step.
- **Risk:** low. The keyword set is the pinned one, and the fingerprint catches any
  mis-tag. Allocations rise by one `Vec` per parse, inside the 2% step tolerance.

### 2. Decide the member once: est. 20–40% of time, most of `peeked`

Read a member's head once: its prefix's extent and the keyword after it. Then dispatch on
that keyword with a `match`, instead of asking 28 recognisers that each re-skip the
prefix. Where several recognisers share a keyword, the `match` arm keeps their original
order.

- **Primary metric:** `peeked`. It should drop by a large factor; 219 per consumed token
  is mostly this.
- **Steps:** likely enabling ones first. Introduce the member-head computation alongside
  the chain (enabling), then switch one dispatcher at a time, `body_element` first.
- **Risk:** medium. The recogniser order encodes priority, including deviation-driven
  choices (ADR-0022). The fingerprint makes any change of choice a rejected step, which
  is exactly the guard this needs.

### 3. Memoize prefix and name skipping: est. 10–20%, a fallback for #2

If #2 is too invasive for a pass, cache what `skip_*_prefix` and `qualified_name_length`
return per start index (a `Cell`-held, lazily filled table on `Parser`). The chain stays,
but each repetition becomes a lookup.

- **Primary metric:** `peeked`, then `time`.
- **Risk:** low to medium. A cache keyed on index must be invalidated by nothing, which
  holds because tokens never change during a parse. The cost is a few `Vec`s of
  allocation.

Do #3 only if #2 is abandoned. Done after #2, there would be little left to save.

### 4. Leave alone for now

- **rowan `NodeCache` (~6%).** It deduplicates green nodes, which is how the tree is
  built. A cache shared across parses would help the editor's reparse loop more than a
  single parse; it belongs with ADR-0013's incremental work, not here.
- **Lexer (2.6%), allocation count.** 9,891 allocations for 73 KB is mostly green nodes.
  Nothing is worth a series until #1 and #2 have moved the rest.

## Suggested order

Do #1, then #2, each as its own series. After #1, re-run this profile. With the string
compares gone, #2's share will be larger and easier to read.
