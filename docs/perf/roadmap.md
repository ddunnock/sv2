# Parser optimization roadmap

Seven phases, each sized to be planned and executed on its own. The evidence is in
[assessment.md](assessment.md); the rules every phase runs under are ADR-0024 and the
`optimize-parser` skill.

**Order:** 0 → 1 → 2 → 4 → 5 → 6, with 3 slotted in whenever a short session is free.
There is a re-profile checkpoint after Phase 2.

## Standing guards (every phase)

Every optimization phase is **one series** (`scripts/perf.sh series open … --primary …`).
Every step in it:

- keeps `./scripts/gate.sh` green, judged on its own exit code;
- moves no snapshot. An optimization that moves one is wrong, not due for review;
- leaves the output fingerprint unchanged: every corpus tree, error and deviation note.
  `series step` enforces this;
- improves its primary metric, or is `--enabling` with the later step named, at most
  three in a row;
- worsens no counter past 2%, and median time past 5% on the same host.

A series closes on a net win, or is abandoned with a reason and reverted. Either way, the
outcome is recorded against its phase here.

The two measurement phases (0 and 1) are not series. They change no parser behaviour, and
the fingerprint check confirms it with the `counters` feature on and off.

## The bottleneck in one paragraph

Parsing the 73.5 KB SimpleVehicleModel takes 8.7 ms median natively. About 70% of that is
lookahead deciding, again, things it has already decided:
- what a keyword is, by string comparison against a 173-entry table;
- where a member's prefix ends, re-skipped by each of 28 recognisers;
- how long a qualified name is, re-measured per recogniser.

The CPU counters agree. Two-thirds of cycles are useful work, so the processor is not
starved; it is doing redundant work, and the cure is doing less of it. Allocations are
mostly the tree itself. Work grows linearly with input: the cost is a constant factor,
not a quadratic.

**Where FIT-4 stands (Phase 1, measured).** Under WebAssembly the largest corpus file
parses at 11.3 ms p95 in JavaScriptCore (Bun) and 12.5 ms in wasmtime: 1.2–1.4× native,
inside the 16 ms budget by 3.5–4.7 ms. So the optimizations are not rescuing a missed
budget. They buy headroom for three things this measurement does not cover:
- the standard library's largest file, which is not vendored. At the measured
  throughput, 16 ms fits about 100 KB;
- the webview's main-thread contention;
- the grammar work still to come, which the ratchet shows adds cost.

Phases 2 and 5 together, at their estimates, would put the largest file near 6 ms under
JavaScriptCore.

---

## Phase 0 — Exact attribution

**Goal.** Replace sampled, inlining-smeared caller attribution with exact call counts
per hot helper, so every later phase sets its target from counts.

**Evidence.** Release inlining credits `skip_one_prefix_metadata` with 10.7% self time,
yet it returns at once unless the token is `#` (`parser/metadata.rs:38`). The sampled
callers are approximate, so targets set from them would be guesses.

**Approach.**
1. Add named counters to `crates/sv2-syntax/src/counter.rs`, behind the `counters` feature:
   - `keyword()` calls, and table entries compared;
   - `is_name` calls;
   - `nth_is_keyword` calls;
   - `qualified_name_length` calls;
   - calls per `skip_*_prefix` helper;
   - recogniser invocations per dispatcher (`at_sysml_keyword_member`, `body_element`).
2. Have `perf_probe` emit them under a `detail` object. `scripts/perf.py` prints them in
   `measure` and records them in the baseline, but does **not** ratchet them.
3. Record the counts in assessment.md, with each one's share per consumed token.

**Not a series.** Behaviour is unchanged. The fingerprint must match with the feature on
and off, and timing is unaffected because timing builds exclude the feature.

**Exit.** assessment.md has an exact count table. **Abandon:** not applicable.

**Outcome.** Done in 9768f88; the counts are in assessment.md, "Exact counts". They
re-scoped Phase 4: qualified names are not hot (2.5 measurements per consumed token), so
the sampled 12.3% was inlining smear, and prefix skipping is what repeats. Every target
below is now set from counts.

**Its plan must decide.** Static counter names or an enum-indexed array; whether
per-recogniser counts need a macro to stay readable.

**Depends on.** Nothing.

---

## Phase 1 — Measure FIT-4 for real

**Goal.** Replace "WebAssembly is probably 1.5–2× native" with a measurement, so the
roadmap knows how far Phases 2–5 must go.

**Evidence.** FIT-4 (ADR-0013) is p95 under 16 ms *in the editor's wasm build* on the
largest standard library file. Today the largest corpus file is 9.9 ms p95 native, and
nothing measures wasm: `sv2-wasm` is a stub and no wasm target is installed.

**Approach.**
1. One-time machine setup, documented for macOS and RHEL 9: `rustup target add
   wasm32-wasip1` and `wasmtime`. This is a download at setup time; nothing fetches at
   build or gate time (invariant 6).
2. Build a timing harness for `wasm32-wasip1` that parses the five largest corpus files,
   and report median and p95 next to the native numbers.
3. Record native vs wasm in this file and in assessment.md, with FIT-4's margin stated.

**Not a series.**

**Exit.** A measured wasm p95 for the largest file, and the margin to 16 ms.
**Abandon:** if wasmtime's timing proves too noisy, record the spread and fall back to
native × the measured ratio.

**Its plan must decide.**
- Reuse `perf_probe` (std I/O works under WASI), or write a dedicated example.
- Whether `perf.sh` gains a `--wasm` timing mode.
- Whether wasmtime's JIT is a fair stand-in for the webview's engine. The plan should
  note it is an approximation.

**Depends on.** Nothing. It runs before Phase 2 so the target is known.

**Outcome.** Done in 82e3051, measured with `scripts/perf.sh wasm` on the MacBook.
The figures are the largest file, 30 runs after 3 warm-ups, and agreed within 1% on a
second run:

| Engine | Median | p95 | Ratio to native | FIT-4 margin |
|---|---|---|---|---|
| native | 8.58 ms | 8.79 ms | — | 7.21 ms |
| wasmtime 49 (Cranelift) | 12.26 ms | 12.42 ms | 1.43× | 3.58 ms |
| Bun 1.4.2 (JavaScriptCore) | 10.64 ms | 11.32 ms | 1.24× | 4.68 ms |

The ratio holds at 1.37–1.43× (wasmtime) and 1.21–1.29× (Bun) across the five largest
files, so it is a property of the engine, not of a file. The roadmap's guess of
1.5–2× was pessimistic.

- **Bun is the closer stand-in.** JavaScriptCore is the engine of Tauri's webview on
  macOS and Linux, but not its exact tiering, nor a main thread shared with layout and
  input.
- **wasmtime is the reproducible reference,** not the editor.
- **Bun's p95 spreads more on small files** (2.4 ms against a 0.93 ms median), consistent
  with JavaScriptCore's tiering and garbage collection.

How it was built:
- The probe's timing mode is compiled to `wasm32-wasip1`, and the WASI guest sees only a
  temporary copy of the timed files, never the repository.
- `proptest` became a non-WASI dev-dependency (its process forking has no WASI
  implementation).
- `rust-toolchain.toml` now installs the target with the toolchain, and STD-002-RS 0.2.1
  states it.

---

## Phase 2 — Classify keywords once

**Goal.** Stop deciding keyword identity by string comparison.

**Evidence.**
- `nth_is_keyword` is 17.9% self time; `memcmp` underneath it is 14.1%.
- `is_name` → `keyword()` (`parser/lookahead.rs:20`) scans all 173 `KEYWORDS` linearly.
  A real name matches none, so every name check is a full scan.
- There are 512 string-literal keyword call sites.
- `KEYWORDS` is already emitted sorted (`scripts/gen_syntax_kinds.py:2360`).

**Approach.**
1. `keyword()` becomes binary search over the sorted table, or a generated `match`
   emitted by `gen_syntax_kinds.py`. The generated file is never hand-edited.
2. *(enabling)* Build a keyword-kind table per token in `Parser::new`, beside
   `meaningful`: one `Vec`, inside the 2% allocation tolerance.
3. `is_name` reads the table: O(1).
4. `nth_is_keyword` compares kinds.
5. *(optional, one module per step)* Migrate the `"…"` call sites to `SyntaxKind::Kw*`.

**Series.** Primary `time`. Estimate: 25–35% of median time. Enabling budget: 1 (step 2).

**Target, from Phase 0's counts (largest file).**
- `keyword()` compares 164 table entries per lookup (4,126,749 over 25,091 lookups).
  Binary search makes that 8 or fewer, and a per-token table makes `is_name` compare none.
- `nth_is_keyword` runs 1,233,818 times, each a string compare. This phase leaves the
  count alone (Phase 5 cuts it) but makes each call a kind comparison.
- Expect `keyword_entries` per `keyword_lookups` ≤ 8 after step 1.

**Quality guards.**
- `every_keyword_this_parser_names_is_in_the_pinned_token_set` (`parser.rs` tests) must
  stay meaningful. If migration makes it compile-time, changing that test is its own
  reviewed change (`.claude/rules/tests.md`: never weaken a test).
- A keyword must still lex as `BasicName` and be tagged by `bump_as`, so the tree is
  unchanged. The fingerprint enforces this.

**Exit.** Net win on time. **Abandon:** step 3 shows under 10% on median time.

**Its plan must decide.** Binary search or a generated `match`; where the table lives
(`Parser` field vs a `Token` extension, given that `Token` is public API); whether step 5
is in scope.

**Depends on.** Phase 0 for the target; Phase 1 for the margin.

**Outcome.** Series `keyword-classification`, closed: **net −25.2% median time** over
the five largest files, measured on the MacBook.

| Step | Change | Verdict |
|---|---|---|
| 1 (77f0464) | `keyword()` binary-searches the sorted table instead of scanning it | improved, −16.7% |
| 2 (201f3d1) | `nth_is_keyword` compares the token's bytes instead of a boundary-checked `str` slice | improved, −10.2% vs step 1 |

Largest file, p95, before → after:

| Engine | Before | After | FIT-4 margin |
|---|---|---|---|
| native | 9.08 ms | 7.68 ms | 8.32 ms |
| Bun (JavaScriptCore) | 11.21 ms | 8.34 ms | 7.66 ms |
| wasmtime | 12.88 ms | 9.40 ms | 6.60 ms |

- `keyword_entries` fell from 4,126,749 to 225,819: 9 probes per lookup, the standard
  library's binary-search bound for 173 entries.
- The ratcheted counters and the output fingerprint did not move at any step.

Notes:
- **Correction:** `KEYWORDS` has 173 entries, not 217. The first count included the
  operator table's rows.
- **Revised estimate:** 25–35% → 10–20% when planned, from Phase 0's counts. The result,
  25%, beat the revision, because step 2's boundary-check saving was larger than the
  compare it rode along with.
- **Step 3 not taken.** The kind-based migration of the 512 call sites was conditional on
  a string compare under `nth_is_keyword` still costing 5% or more after step 2. The
  re-profile showed none separately, so Phase 5 is left to cut the calls themselves.

**Checkpoint, done after the close.** Instruments Time Profiler self time, with system
`memcmp` attributed to its caller:

| Self | Where |
|---|---|
| 18.5% | `skip_basic_usage_prefix` (inlines `nth_is_keyword` nine times) |
| 16.5% | `nth_is_keyword` |
| 10.6% | `memcmp` inside `keyword()`'s binary search |
| 5.3% | `skip_one_prefix_metadata` |
| 4.1% | rowan `node_hash` |
| ~6% | `tokenize` and its `memcmp` (Phase 3) |
| 2.8% | `skip_occurrence_usage_prefix` |

Re-ranked:
- **Phase 4 stays next.** The three prefix skips hold about 27% of self time, and each
  recomputes from the same index.
- **Phase 5 follows,** cutting the `nth_is_keyword` calls that remain.
- **A new candidate, "2b",** comes out of this checkpoint: `keyword()`'s remaining 10.6%.
  Two ways to cut it:
  - a keyword-kind table per token, built once in `Parser::new`, so `is_name` stops
    looking names up at all;
  - a generated `match` on the text, which Rust compiles to length-first comparisons.

  Plan it as its own series after Phase 4. Phase 4's caches will change how often
  `is_name` is reached.

**2b, re-planned (2026-10-05, after Phase 5 and keyword-table-per-language).** The target
moved twice:
- Phases 4 and 5 cut `keyword_lookups` over the corpus from 195,225 to 16,129.
- `is_name` stopped calling `keyword()` (621f1b2), and binary-searches the file's own
  `RESERVED_*` table instead.

Measured at bffb492, `perf` self time on the whole-corpus bench (RHEL 9 workspace, under
load, so shares only):

| Self | Where |
|---|---|
| 16.2% | `nth_is_keyword` (1,912,055 calls; 174 per member decision) |
| 3.3% + 2.3% `memcmp` | `is_reserved` (51,727 `is_name` calls) |
| 2.8% | `KeywordMember::admits` (string compares on the head word) |
| 1.3% + 1.1% `memcmp` | `keyword()`, and `nth_is_keyword`'s own compare |

About 27% of self time is spent asking the same question: which keyword, if any, is
this token? The tokens never change during a parse, so the question has one answer per
token.

Decisions:
- **Classify in the lexer, not in `Parser::new`.**
  - `Token` gains `keyword: Option<SyntaxKind>`, set by `tokenize` for each `BasicName`
    through one `keyword()` lookup.
  - `Token` is a `u16` and two `usize`s, 24 bytes with 6 of padding. `Option<SyntaxKind>`
    is 2 bytes by the enum's niche, so `Token` stays 24 bytes. That means no allocation
    and no `allocated_bytes` growth; a side table built in `Parser::new` would cost both.
  - The classification is language-independent, so it belongs to the lexer. Nothing
    outside the lexer constructs a `Token`.
- **Reserved as a kind question.** The generator emits `reserved_in_kerml(kind)` and
  `reserved_in_sysml(kind)` as `matches!` over keyword kinds, beside the word tables.
  `is_name` becomes a kind test, with no search and no `memcmp`.
- **The 343 `nth_is_keyword(n, "word")` call sites stay as they are** in this series.
  Migrating them to kinds is a larger, mechanical change, and a series of its own if a
  re-measure still shows the compare.

Steps:
1. `Token::keyword` set in `tokenize`, and `is_name` answers from it through the generated
   reserved-kind tests. `is_reserved`'s binary search goes.
2. `nth_is_keyword` early-outs on a token whose `keyword` is `None`: a name is never a
   keyword, so it needs no byte compare.
3. `KeywordMember::admits` and `member_head` compare the head's keyword kind rather than
   its text.

**Series.** Primary `time`, since no counter sees a compare, so it needs this
workspace quiet. Enabling budget: 0, because step 1 lands with its reader. **Guards:**
- The output fingerprint must not move.
- `Token`'s size is asserted to stay 24 bytes.
- `reserved_words_are_sorted_and_keywords` is extended to the kind tests: each must
  agree with its word table for every keyword.

**Exit:** a net time win. **Abandon:** under 3% after step 2.

**Follow-up.** Series `keyword-lookahead` took the calls instead of the compare: one
peek per group of words at 21 sites. It closed at **−15.5% `peeked`** (−3.3%
instructions); see assessment.md, "Keyword lookahead, by caller".

**Outcome.** Series `token-keywords`, **abandoned** at its own line. Steps 1–2 cut
instructions by about 1.4%. The text compare was never the cost; see assessment.md,
"Keyword classification by token (2b)". The step-1 code was never committed and was
backed out.

---

## Phase 3 — Lexer operator dispatch

**Goal.** Stop scanning the operator table at every operator position.

**Evidence.** `lex_operator` (`lexer.rs:281`) tries every `OPERATORS` entry longest-first
with `cursor.eat`. `memcmp` under `tokenize` is 2.8%, and `tokenize` itself is 2.6%.

**Approach.**
1. Dispatch on the first byte to the candidates starting with it, still longest first.
   Ideally the generator emits these groups, so the table stays the single source.

**Series.** Primary `time`. Estimate: 3–5%. Enabling budget: 0.

**Quality guards.** The lexer roundtrip proptest and `tests/lexer.rs`. Maximal munch
(`::>` before `::` before `:`) must be preserved exactly; the fingerprint catches any
change in token kinds.

**Exit.** Net win on time. **Abandon:** under 2%. It is not worth a generator change.

**Its plan must decide.** Generated grouping vs a hand-written `match` on the byte.

**Depends on.** Nothing. Any time.

**Outcome.** Series `operator-dispatch`, closed: **net −3.2% median time** (series
ledger, RHEL 9 workspace, where it was opened and closed). One step, improved, no
enabling.

| Step | Change | Verdict |
|---|---|---|
| 1 (694e01f) | `OPERATORS_BY_FIRST_BYTE`, and `lex_operator` compares each byte's candidates inline on the source bytes | improved, −3.2% |

- **Neither generated grouping nor a hand-written `match`.** A `const fn` derives the
  per-byte index from `OPERATORS` at compile time. The generated table stays the single
  source, and each byte's candidates keep its longest-first order. More than 8 operators
  sharing a first byte (6 share `:`), a non-ASCII operator, or a table too long for a
  `u8` index is a build error.
- **The first form was rejected,** at −2.7%: first-byte dispatch over `<[u8]>::starts_with`.
  The rework compares the at-most-three bytes inline, which removes a `memcmp` call per
  candidate, and reads the source bytes without `Cursor::rest`'s `str` boundary check.
  That is a different change, not a re-run.
- **The lexer alone halved.** In a throwaway A/B of `tokenize` over the corpus, five
  rounds alternating old and new, the median went from about 4.8 to 2.65 ms (−45%).
  The parse-level −3.2% sits at this workspace's noise floor; the lexer-level number is
  the clear signal.
- `operator_dispatch_agrees_with_the_full_scan` compares the new lexer with the old scan
  on every string of up to three characters over the operator alphabet, plus a letter, a
  space and a non-ASCII character. A negative control that skipped each row's longest
  candidate failed it on `!==`.
- What remains of `tokenize` is char decoding in `Cursor::peek`/`eat_while` and the
  growth of its `Vec`. That is a byte-level lexer, and a hypothesis of its own.

---

## Phase 4 — Memoize lookahead scans

**Goal.** Compute each prefix extent once per start position.

**Evidence.** Phase 0's counts, per member decision (largest file / corpus):

| Helper | Calls | Per decision |
|---|---|---|
| `skip_prefix_metadata` | 138,283 / 2,228,462 | 130.7 / 202.4 |
| `skip_basic_usage_prefix` | 70,192 / 1,210,455 | 66.3 / 110.0 |
| `skip_occurrence_usage_prefix` | 57,320 / 972,243 | 54.2 / 88.3 |
| `qualified_name_length` | 17,763 / 119,868 | 16.8 / 10.9 |

The prefix skips are pure functions of their start index over tokens that never change
during a parse, and each runs dozens to hundreds of times per member.

**Qualified names are out of this phase.** At 2.5 measurements per consumed token there
is little to save. The sampled profile's 12.3% for `qualified_name_length` was inlining
smear.

**Approach.** One helper per step, in order of calls:
1. *(enabling)* A per-index cache on `Parser`: `Cell`-held, filled lazily, sized once from
   the meaningful-token count.
2. `skip_prefix_metadata` reads through it.
3. `skip_occurrence_usage_prefix` reads through it, which also covers most
   `skip_basic_usage_prefix` calls.
4. `skip_basic_usage_prefix`'s remaining direct callers.

**Target.** Each of the three prefix skips computes at most once per start index. Their
*computations* per member decision fall from 54–202 to a handful. The call count stays,
so measure computations with a detail counter on the cache-miss path.

**Series.** Primary `peeked`. Estimate: 10–20% of time, and a large share of `peeked`.
Enabling budget: 1.

**Quality guards.**
- Size the cache once, so allocation steps stay within 2%.
- A cache keyed by index must account for `comments_significant`, which changes what
  "the nth meaningful token" is. The key must include it, or the cache must bypass
  that mode.

**Exit.** Net win on `peeked`, no time regression. **Abandon:** two enabling-or-flat
steps with `peeked` down under 10%.

**Its plan must decide.** The cache representation (dense `Vec<u32>` with a sentinel vs a
sparse map), and how the `comments_significant` mode is handled.

**Depends on.** The Phase 2 checkpoint for its target. Phase 5 builds on it.

**Plan (2026-10-04).** Read from the code before anything changed:
- The three helpers read only `tokens` (never mutated during a parse) and the comment
  mode. They are pure, so a cache needs no invalidation.
- Their `n` is relative to the cursor (`peek_nth` reads `cursor_position() + n`). The key
  is therefore the absolute meaningful position, `cursor_position() + n`, and the cached
  value is an absolute end, returned less `cursor_position()`.

Decisions:
- **Representation: a small fixed array per helper, not a dense `Vec`.** The largest file
  has about 7,000 meaningful tokens, so one `u32` per token per helper is about 28 KB, and
  84 KB for three. That is about 6% of its 1.31 MB `allocated_bytes`, and every file
  scales the same way, so it breaks the 2% allocation tolerance on the corpus total.
  A `[Cell<Entry>; N]` held on `Parser` allocates nothing. The roadmap's step 1 needed to
  be enabling only because a `Vec` costs allocations, so it folds into the first reader
  and the enabling budget stays unspent. This is right only if the calls for one member
  land on a few nearby positions, so step 0 measures that.
- **`comments_significant` is part of the key.** An entry from one mode is never served
  in the other, with no extra branch.

Steps:
0. *(before the series; not a step)* A throwaway log of every helper call's absolute
   start and mode over the corpus, replayed against 1-, 4-, 8- and 16-slot caches. The
   hit rates, the distinct starts per member decision, and the chosen `N` are recorded
   in assessment.md. Nothing in the parser source is committed, so the series opens
   on a clean tree.
1. The cache type, and `skip_prefix_metadata` reads through it, with a
   `skip_prefix_metadata_computed` detail counter on the miss path.
2. `skip_occurrence_usage_prefix` reads through it (its own counter), which also covers
   most `skip_basic_usage_prefix` calls.
3. `skip_basic_usage_prefix`'s remaining direct callers (three sites and
   `skip_unextended_usage_prefix`).

**Step 0, done.** Each helper starts at about one distinct position per member decision
(0.7–1.1). 8 direct-mapped slots indexed by `position % 8` reach that floor exactly, and
even one slot hits 96–98% (assessment.md, "Prefix-skip start positions"). `N` is 8. The
ceiling for the series: calls that compute fall from 4.41 M to about 28 K over the corpus.

After the close, re-measure "2b" (`keyword()`'s remaining self time), since the caches
change how often `is_name` is reached.

**Outcome.** Series `prefix-skip-memo`, closed: **net −81.9% `peeked`** over the corpus
(23,986,614 → 4,331,571). Median time over the five largest files fell 23.92 → 13.62 ms
(−43%). It was opened, stepped and closed on the RHEL 9 workspace, so every time
comparison is same-host. Three steps, all improved, none enabling:

| Step | Change | Verdict |
|---|---|---|
| 1 (0c9e6af) | `PrefixCache`, and `skip_prefix_metadata` reads through it | improved, `peeked` −11.6% |
| 2 (892ab63) | `skip_occurrence_usage_prefix` reads through its own | improved, −76.4% vs step 1 |
| 3 (0ce49b7) | `skip_basic_usage_prefix` reads through its own | improved, −13.4% vs step 2 |

Calls and computations over the corpus, before → after (`*_computed` detail counters):

| Helper | Calls | Computed |
|---|---|---|
| `skip_prefix_metadata` | 2,228,462 → 595,231 | 12,029 |
| `skip_occurrence_usage_prefix` | 972,243 → 394,233 | 7,906 |
| `skip_basic_usage_prefix` | 1,210,455 → 94,288 | 7,907 |
| `nth_is_keyword` | 20,355,352 → 3,637,378 | — |

- 27,842 computations in all, against the 28 K the step-0 replay predicted.
- Allocations and the output fingerprint did not move at any step.
- **The estimate was low.** It was 10–20% of time, and the result was 43%. A hit on the
  occurrence prefix also skips the basic-prefix and metadata skips nested inside it, and
  those nested calls held most of the `nth_is_keyword` traffic.
- Step 2 was measured twice. The first measurement was recorded over source that failed
  clippy's `excessive_nesting`, so it was discarded before commit and the fix measured
  in its place. Both runs gave −76.4%.

Largest file, p95, after the close (RHEL 9 workspace; not comparable with the MacBook's
numbers, and noisy there):

| Engine | Before (at open) | After | FIT-4 margin |
|---|---|---|---|
| native | 10.5–14.3 ms | 7.12 ms | 8.88 ms |
| Bun (JavaScriptCore) | 14.1–16.0 ms | 9.02 ms | 6.98 ms |
| wasmtime | 15.1–17.7 ms | 9.11 ms | 6.89 ms |

**Checkpoint, done after the close.** `perf` self time on the RHEL 9 workspace,
`simple_vehicle_model` bench:

| Self | Where |
|---|---|
| 22.8% | `nth_is_keyword` |
| 13.8% | `memcmp` (callers not resolved: neither frame-pointer nor DWARF unwinding gets through libc's) |
| 6.5% + 4.7% + 4.2% + 3.8% | rowan `node_hash`, `NodeCache::node`, `NodeCache::token`, `Arc::drop_slow` |
| 5.6% | `tokenize` (Phase 3) |
| 4.8% | `keyword()` ("2b"), before its share of `memcmp` |
| 2.1% | `skip_prefix_metadata`, now mostly cache hits |

Re-ranked:
- **Phase 5 is next.** `nth_is_keyword` is again the largest self time, and its remaining
  3.6 M calls are what dispatching on the member head removes.
- **"2b" stays after Phase 5.** `keyword()` is 4.8% self here, plus an unknown share of
  `memcmp`. Split `memcmp` with Instruments on the MacBook before planning it.
- **rowan's tree building now shows,** at about 19% across the four entries. That is
  Phase 6's territory, and the first time it ranks.

---

## Phase 5 — Dispatch on the member head

**Goal.** Decide what a member is once, from its head, instead of asking 28 recognisers
in turn.

**Evidence.**
- 65% of parse self time sits under an `at_*` recogniser.
- `at_sysml_keyword_member` (`parser/namespace.rs:113`) chains 28 alternatives, and each
  re-skips the same prefix before reading the one keyword that decides it.
- CPU counters: 19.2% of cycles are front-end delivery stalls, consistent with control
  flow hopping between many small functions.

**Approach.**
1. *(enabling)* Compute the member head (prefix extent, plus the keyword or token after
   it), using Phase 4's caches, alongside the existing chain.
2. Switch `at_sysml_keyword_member` to a `match` on the head keyword. Each arm runs only
   the recognisers that keyword can start, **in their original order**.
3. Switch `body_element`'s dispatch the same way.
4. Switch the remaining dispatchers, one per step.

**Target, from Phase 0's counts.**
- Peeks per member decision: 1,444 on the largest file (1,527,604 / 1,058) and 2,179 over
  the corpus.
- `nth_is_keyword` per decision: 1,166 / 1,849.
- `at_sysml_keyword_member` runs 2.0 times per decision on the largest file and 3.3 over
  the corpus, because the chain is re-asked for the same member.

The goal is one dispatch per decision, and peeks per decision at least halved.

**Series.** Primary `peeked`. Estimate: 20–40% of time, and most of the 219-per-token
peek ratio. Enabling budget: 2.

**Quality guards.**
- Recogniser order encodes priority, including deviation-driven choices (ADR-0022).
- Run the `spec-conformance-reviewer` agent on every dispatcher switch.
- The fingerprint rejects any step that changes a choice anywhere in the corpus.
- Keep the negative corpus (`tests/rejection/`) rejecting.
- A keyword-less member (`a : T;`, KerML's keyword-less feature) needs a default arm that
  is exactly the old fall-through.

**Exit.** Net win on `peeked`, no time regression. **Abandon:** enabling budget spent with
`peeked` down under 30%. Record what was learned against this phase.

**Its plan must decide.** The head's representation; whether the arms are generated from
a table or written by hand; how the per-keyword candidate sets are derived and verified
against the old chain (ideally by a test that asserts they agree).

**Depends on.** Phase 4 (cheap head computation). Phase 2 (kind comparison).

**Plan (2026-10-05).** Step 0 is done (assessment.md, "Keyword-member dispatch by head").
The chain is 55% of all peeks, and 68% of that is spent in recognisers that never accept
at the head they were asked at.

Decisions:
- **Head representation.** `MemberHead { at: usize, kind: SyntaxKind }`:
  - `at` is the relative index of the first token after `#` metadata and the prefix
    words. `kind` is that token's keyword kind, or its token kind when it is not a
    keyword.
  - Computed once per `at_sysml_keyword_member` call, through Phase 4's caches.
  - No allocation. It is keyed exactly as the step-0 log was keyed.
- **A table, not a hand-written `match`.** A `Recogniser` enum names the 28 chain
  members. A `const` table maps a head kind to `&[Recogniser]`, and one `match` turns a
  `Recogniser` into its call. A head the table does not list falls back to the whole
  chain, in order. That keeps the keyword-less member and every rare head exactly the old
  fall-through.
- **Candidate sets come from the code, and a test proves them.**
  - Each recogniser's possible heads are read from its own code and cited at its table
    row. They are never taken from the step-0 log.
  - A unit test asserts every row is an ordered subsequence of the chain. Order encodes
    priority, including the choices deviations make (ADR-0022).
  - A **differential test** keeps the old chain under `#[cfg(test)]` and compares it
    with the new dispatch at every meaningful token position, in both comment modes,
    over the positive corpus, `tests/rejection/` and the test fixtures. The answers must
    be identical.
  - The fingerprint holds the same over the corpus at each step.

Steps:
1. Compute `MemberHead`, and ask `at_definition_element` only at the heads that can open
   a definition, a package, a dependency or an annotating element. This is the largest
   single cost: 728 K peeks at heads where it never wins. It is a win, not enabling,
   because it lands with its first reader.
2. Replace the rest of `at_sysml_keyword_member`'s chain with the table. The nested
   calls from `at_end_kind` become cheap here, which is where most of the `end` cost
   goes.
3. Memoize `skip_end_usage_prefix` in a `PrefixCache`, storing `None` as an end equal to
   the start. This removes the remaining repeated walks over a cross feature.
4. *(enabling, if needed)* Decide once. `at_sysml_keyword_member` returns the
   `Recogniser` that accepted, and `membership` dispatches on it instead of re-running
   `definition_element`'s and `usage_element_of_class`'s chains.
5. `body_element`'s own arms on the head, if a re-measure still shows them.

Each dispatcher switch (steps 1, 2, 4, 5) gets a `spec-conformance-reviewer` pass before
its commit.

**Expected.** Steps 1–2: `peeked` −25 to −37% (the step-0 bound is 1.62 M of 4.36 M).
Step 3: most of what remains of the `end` cost. Step 4: the second decision's share,
which step 0 did not isolate.

**Outcome.** Series `member-head-dispatch`, closed: **net −41.1% `peeked`**
(4,331,571 → 2,552,786), against the −25 to −37% expected. Median time over the five
largest files fell from 14.35 ms at the open to about 12.4 ms. The series was opened,
stepped and closed on the RHEL 9 workspace. Five steps, all improved, none enabling:

| Step | Change | Verdict |
|---|---|---|
| 1 (18da45a) | `MemberHead`; `at_definition_element` asked only where `opens_definition` admits the head | improved, −16.4% |
| 2 (4e1f85b) | `KeywordMember` and `admits`: every keyword member asked only where its head admits it, in the chain's order | improved, −17.3% |
| 3 (cb9858f) | `skip_end_usage_prefix` memoized, `None` stored as an end equal to the start | improved, −2.8% |
| 4 (0f47596) | `membership` re-runs `definition_element` only at a definition head | improved, −5.8% |
| 5 (bc06618) | `membership` re-runs the three usage chains only where `admits_here` holds | improved, −6.8% |

How it differs from the plan:
- **Gates, not a table.** Each `KeywordMember` gets an `admits` predicate on the head,
  and the chain is walked in its original order. That keeps priority by construction,
  where a table of candidate lists would need a test to keep each list ordered.
  `MemberHead` records three flags: `ref`, `individual`/`snapshot`/`timeslice`, and `#`
  metadata. Those three members decide on a prefix that the head skips.
- **"Decide once" became step 4 and step 5's gates.** Before step 4, a throwaway count put
  `membership`'s second decision (from its member prefix to its first consumed token) at
  1,149,945 of 2,908,282 peeks. Its chains run in a different order from the keyword
  chain, so passing the first decision through would mean mapping one priority onto
  another. The same head checks gate the second decision instead. Steps 4 and 5 removed
  about 355 K of it. The rest of that window is mostly lookahead inside the chosen
  production before it consumes anything, not dispatch.
- **Step 3 was small** (−2.8%), because step 2 had already made the nested dispatch inside
  the `end` walk cheap.
- **`body_element`'s own arms** (the old step 5) were not taken. Step 0 measured
  `at_result_expression` at 3% and `at_source_succession_member` at 0.3%.

The evidence for every gate is two implication tests, run at every meaningful position of
the corpus, `tests/rejection` and a list of hand-written forms, in both comment modes:
- `definition_element_heads_cover_every_definition`: 609 files, 4,782 definitions.
- `every_keyword_member_is_admitted_where_it_accepts`: 646 sources, 16,226 acceptances.

Negative controls on both failed them, as they should. **The tests are empirical, not a
proof.** A recogniser that accepted on input outside those sources at a head its
`admits` arm rejects would change meaning silently. A new recogniser, or a new keyword
in one, needs its `admits` arm updated and a form added. Each dispatcher step had a
`spec-conformance-reviewer` pass: 0 blocking in all four.

Largest file, p95, after the close (RHEL 9 workspace, two runs; take FIT-4 on the
MacBook):

| Engine | After Phase 4 | After Phase 5 | FIT-4 margin |
|---|---|---|---|
| native | 7.12 ms | 5.2 ms | 10.8 ms |
| Bun (JavaScriptCore) | 9.02 ms | 8.6–9.0 ms | ≥ 7.0 ms |
| wasmtime | 9.11 ms | 6.0–7.5 ms | ≥ 8.5 ms |

**Self time after the close** (`perf`, whole-corpus bench). DWARF unwinding now resolves
`memcmp`'s caller, though inclusive time through the parser's recursion still truncates:

| Self | Where |
|---|---|
| 14.2% | `nth_is_keyword` |
| ~11% | `keyword()`'s binary search, `memcmp` included ("2b") |
| ~9% | `tokenize` (Phase 3) |
| ~20% | rowan tree building and dropping (Phase 6) |

Carried forward:
- The leading comments on `non_occurrence_usage_element`, `structure_usage_element` and
  `behavior_usage_element` should point at `admits_here`. This is step 5's review
  advisory, left for the next change to the parser, because a comment-only change cannot
  be measured as a step.
- **"2b" is now the largest single target,** together with pending decision
  `keyword-table-per-language`.

---

## Phase 6 — Allocation trims

**Goal.** Trim allocation that is not the tree itself.

**Evidence.**
- 9,891 allocations and 1.31 MB for the largest file (`tests/perf-baseline.json`).
- Instruments Allocations: 69% of allocations are 48- or 64-byte blocks. By size, those
  are rowan green nodes, which are the tree and are not reducible without changing its
  shape. That is forbidden.
- What remains is buffers: the token `Vec`, the two meaningful-index `Vec`s, the builder's
  child stacks, and diagnostic strings.

**Approach.**
1. A capacity hint for `tokens`, from the source length.
2. Merge `meaningful` and `meaningful_with_comments`, or derive one from the other.
3. Anything Phase 0's counts and a fresh Allocations trace show above 1% of
   `allocated_bytes`.

**Series.** Primary `allocated_bytes`. Estimate: modest, 5–15% of bytes and little time.
Enabling budget: 1.

**Quality guards.** As standing.

**Exit.** Net win on `allocated_bytes`. **Abandon:** under 5%.

**Out of scope.** A rowan `NodeCache` shared across parses, which would cut node hashing
and allocation for repeated parses. It belongs to ADR-0013's incremental-reparse work and
is recorded there as a candidate, not here.

**Depends on.** Phase 0.

---

## Outcomes

| Phase | Series | Outcome | Notes |
|---|---|---|---|
| 0 | — | done, 9768f88 | counts in assessment.md; re-scoped Phase 4 to prefix skips |
| 1 | — | done, 82e3051 | largest file p95: 11.3 ms JavaScriptCore, 12.5 ms wasmtime; within FIT-4 by 3.5–4.7 ms |
| 2 | keyword-classification | closed, net −25.2% time | Bun p95 11.21 → 8.34 ms; two steps, no enabling |
| 3 | operator-dispatch | closed, net −3.2% time | one step; `tokenize` alone −45% |
| 4 | prefix-skip-memo | closed, net −81.9% peeked | median time 23.92 → 13.62 ms (RHEL 9); three steps, no enabling |
| 5 | member-head-dispatch | closed, net −41.1% peeked | five steps, no enabling; gates by `admits`, not a table |
| 6 | | not started | |
