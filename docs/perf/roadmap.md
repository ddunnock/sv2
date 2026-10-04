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
- what a keyword is, by string comparison against a 217-entry table;
- where a member's prefix ends, re-skipped by each of 28 recognisers;
- how long a qualified name is, re-measured per recogniser.

The CPU counters agree. Two-thirds of cycles are useful work, so the processor is not
starved; it is doing redundant work, and the cure is doing less of it. Allocations are
mostly the tree itself. Work grows linearly with input: the cost is a constant factor,
not a quadratic.

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

---

## Phase 2 — Classify keywords once

**Goal.** Stop deciding keyword identity by string comparison.

**Evidence.**
- `nth_is_keyword` is 17.9% self time; `memcmp` underneath it is 14.1%.
- `is_name` → `keyword()` (`parser/lookahead.rs:20`) scans all 217 `KEYWORDS` linearly.
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

**Checkpoint.** After Phase 2, re-run the flamegraph and the Phase 0 counts. With string
compares gone, the shares of Phases 4 and 5 will be larger and clearer. Re-rank them here
before planning Phase 4.

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

---

## Phase 4 — Memoize lookahead scans

**Goal.** Compute each prefix extent and qualified-name length once per start position.

**Evidence.**
- `skip_basic_usage_prefix` is 13.7% self, reached through
  `skip_occurrence_usage_prefix` (24 call sites).
- `qualified_name_length` is 12.3% self.
- Both are pure functions of the start index over tokens that never change during a parse.
- 219 tokens are peeked per token consumed on the largest file; 340 over the corpus.

**Approach.** One helper per step:
1. *(enabling)* A per-index cache on `Parser`: `Cell`-held, filled lazily, sized once from
   the meaningful-token count.
2. `skip_occurrence_usage_prefix` reads through it.
3. `qualified_name_length` / `skip_qualified_name` read through it.
4. `skip_prefix_metadata` reads through it.

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
| 0 | — | not started | |
| 1 | — | not started | |
| 2 | | not started | |
| 3 | | not started | |
| 4 | | not started | |
| 5 | | not started | |
| 6 | | not started | |
