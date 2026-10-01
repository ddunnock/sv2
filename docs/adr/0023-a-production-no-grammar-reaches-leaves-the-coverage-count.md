---
title: "A production no grammar reaches is recorded as unreachable and leaves the coverage count"
status: proposed
date: 2026-10-01
deciders: [David]
---

# A production no grammar reaches is recorded as unreachable and leaves the coverage count

Amends one sentence of ADR-0015, and touches nothing else in it.

## Context and problem statement

ADR-0015 gives every production the units the grammars that reach it need, and says of
the rest: "A production neither grammar reaches keeps the language that states it." So a
production referenced by no other production is still a live unit, counted in the
coverage denominator, and can never be marked: there is no text that reaches it, so no
parser method reads it and no test can exercise it.

One such production remains. `OwnedExpressionReferenceMember` (KerML 8.2.5.8.1,
`ownedRelationship += OwnedExpressionReference`) is defined in the specification BNF and
referenced by no production in either language's BNF or in the Pilot, which has no rule
of that name. `OwnedExpressionReference` itself is reached, through
`ArgumentExpressionValue`. The grammar-adjudicator found no production it could be a typo
for, as `NonFeatureChainPrimaryArgument` turned out to be (deviation
NonFeatureChainPrimaryArgumentMember). Its register entry today is `spec_only`/
`follow_spec`, "implement it", which cannot be done observably.

Left as it is, coverage reports 553 of 554 for ever, and the one is not a gap in the
parser but in the grammar.

## Decision drivers

- Coverage is a claim about the parser. A unit no input can reach says nothing about it.
- The denominator must not be edited to clear the gate (the working rule "fix the rule,
  never the check"). An exclusion has to be a reviewed decision, recorded per production
  with evidence, not a list in the checker.
- The register is already the place reviewed departures from the BNF live (ADR-0022).

## Considered options

1. **Record it as `unreachable` in the register and exclude it from the denominator.**
2. **Leave it a live unit**, a permanent, explained gap.
3. **Write a reader for it and mark that**, reading text nothing in the language leads to.

## Decision outcome

Option 1. The register gains the decision `unreachable`: a production the specification
defines that neither grammar reaches from its root, recorded with evidence that nothing
references it. `scripts/bnf_coverage.py` leaves a unit whose production carries that
decision out of the denominator, and reports how many it left out, so the exclusion is
visible in every run rather than silent. A marker on an excluded unit is still accepted,
because the unit still exists.

The ADR-0015 sentence becomes: a production neither grammar reaches keeps the language
that states it, and if the register records it `unreachable` it is not counted.

### Consequences

- Good: coverage measures the parser against what the language can say.
- Good: the exclusion is per production, evidenced, and reviewed, and the count is
  printed.
- Bad: a reachability judgment made by hand can be wrong. The first one this repository
  considered, `NonFeatureChainPrimaryArgument`, was wrong: it was a BNF typo, and the fix
  made it reachable. Every `unreachable` entry must therefore cite the search that found
  no reference, in both BNFs and the Pilot, and the grammar-adjudicator's view on whether
  it is a typo for something reached.
- Bad: a re-pinned BNF that starts referencing the production would leave it excluded
  until the entry is revisited. `grammar_plan.py`'s reachability report is where that
  would show.

## More information

- `.claude/state/schema/deviations.schema.json` gains `unreachable` in the decision
  enum. The schema is edited by the author, not in an agent turn.
- ADR-0015 ("A production belongs to the grammars that reach it") is amended in the one
  sentence quoted above.
