---
paths:
  - "**/tests/**"
  - "**/*_test.rs"
  - "**/*.snap"
  - "tests/**"
---

# Tests

## Expectations come from the spec or the corpus, never from output

Do not run the code, look at what it produced, and write that down as the expectation.
That test proves the code does what it does. It cannot detect drift, which is the only
reason this suite exists.

Every expectation traces to one of:

- a spec clause, retrieved via the wiki navigator, cited in a comment
- a corpus file that a conformant tool accepts or rejects
- an explicit local deviation already recorded in `.claude/state/deviations.json`

## Never weaken a failing test

Not by narrowing the assertion, not by `#[ignore]`, not by deleting the case, not by
relaxing a matcher until it passes. A failing test is a real defect or a spec misreading.
Both are fixed in the code or in the understanding.

If the test is genuinely wrong, say so out loud, state what the spec actually requires,
and fix the test as its own change with the citation attached — never bundled into the
change that made it fail.

## The negative corpus is what makes acceptance mean anything

A parser that accepts everything passes a positive-only sweep. `tests/rejection/` holds
inputs that must be rejected, each named for the rule it violates and each carrying a
clause citation in a header comment. Add negative cases in the same change as the
positive ones.

## Snapshots

One snapshot change per code change, reviewed individually. If a snapshot moved and you
cannot name the code change that moved it, stop — something else changed too.

Refresh snapshots in the same commit as the code, never in a follow-up "update snapshots"
commit, which hides drift in a diff nobody reads.

## Round-trip is a property test, not an example test

`parse(s).text() == s` over generated and corpus inputs. This is the single test that
protects the losslessness invariant, so it runs on every corpus file, not on a handful of
hand-written strings.
