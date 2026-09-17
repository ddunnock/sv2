---
title: "A production belongs to the grammars that reach it"
status: accepted
date: 2026-09-17
deciders: [David]
---

# A production belongs to the grammars that reach it

Refines ADR-0014, which remains in force.

## Context and problem statement

ADR-0014 split a production into a KerML and a SysML variant when the two languages state
it differently, and made every other production a shared unit that belongs to both
grammars. "Every other production" turned out to be most of the grammar for the wrong
reason. Of the 558 names in the Tier B′ inventory, 207 are stated only in KerML and 268
only in SysML. All of them were shared, so the SysML grammar contained KerML's `class`,
`assoc` and `behavior` declarations, and the KerML grammar contained SysML's `part def`.
The oracle could not have caught a construct from one language appearing in the other's
file, and catching that is what ADR-0014 exists to guarantee.

A production stated in one language cannot simply be given to that language, though.
SysML states no expression productions of its own: it uses "the expression notation from
[KerML, 7.4.9]", and SysML 8.2.2.17.4 names argument productions that "are the same as
given in [KerML, 8.2.5.8.1]". A KerML-only production can belong to both grammars.

Following those references literally carries far more than the expression layer into
SysML. Two references in SysML's reading of the text reach KerML's whole kernel:

- SysML 8.2.2.4.1 writes `AnnotatingElement = ... | MetadataFeature`, KerML's production,
  whose body admits KerML's NonFeatureMember.
- SysML does not state `ExpressionBody`, so it falls back to KerML's
  `'{' FunctionBodyPart '}'`, whose TypeBodyElement reaches the kernel.

## Decision drivers

- ADR-0014's reason for existing: the oracle must be able to fail on a construct from the
  wrong language
- Invariant 4: where the grammar departs from the text, a recorded deviation says why
- Verified units are reviewed decisions and are not discarded when only their scope moves

## Considered options

1. **Scope by the file that states the production.** KerML-stated productions become
   `@kerml`, SysML-stated become `@sysml`.
2. **Scope by reachability, with a recorded boundary.** A production belongs to the
   grammars that reach it from their root. SysML falls back to KerML's production where it
   states none. The two references that leak the kernel are recorded as deviations and cut.
3. **Keep ADR-0014's rule.** Everything not stated differently stays shared.

## Decision outcome

Option 2.

**Scopes are computed, not chosen.** `production_scopes` in `_grammar.py` gives each
production its unit scopes from the Tier B′ inventory and the boundary, deterministically:

- stated differently in the two languages, or named in the boundary: split into both
  variants (ADR-0014)
- stated identically in both: shared
- stated in one language: shared if both grammars reach it from `RootNamespace`,
  otherwise that language's variant alone. A production neither grammar reaches keeps
  the language that states it.

**Reach follows the text.** KerML reads only KerML's productions. SysML reads its own,
and KerML's for any it does not state. References are taken from Tier B′ bodies with
literals removed.

**The boundary is two recorded deviations.** `SYSML_BOUNDARY` in `_grammar.py` replaces a
production's body in SysML, and each entry names its deviation in `deviations.json`:

- `AnnotatingElement`: SysML reads `MetadataUsage` for `MetadataFeature`. SysML 8.3.27.3
  makes MetadataUsage a MetadataFeature, and SysML 8.2.2.27 already restates the sibling
  productions PrefixMetadataAnnotation and PrefixMetadataMember over MetadataUsage.
- `ExpressionBody`: SysML reads its own `CalculationBody` (8.2.2.19). The SysML corpus
  writes `forAll {in ref w; ...}`, and `ref` is a SysML usage prefix that KerML's body
  cannot parse.

Both were adjudicated, and both agree with the pilot SysML grammar. Neither is filed with
OMG.

**Re-scoping carries work.** When a shared unit gives way to a variant whose inputs hash
the same, its rule, decision and evidence carry over, and it is verified again against its
own grammar. A single-language variant whose inputs differ keeps its rule as the previous
one and goes back to pending. A split variant whose inputs differ starts fresh, as
ADR-0014 splits do. The shared unit is retired with the reason.

### Consequences

Good: KerML's kernel is no longer in the SysML grammar, nor SysML's usages in KerML's. At
the time of this record the plan gives 131 shared units, 100 KerML-only, 267 SysML-only,
and 28 productions split into two variants. Of the 227 units verified before, 226 carried
over and were verified again against their own language's grammar; the 68 that moved went
through `derived` first. The one that did not carry is `AnnotatingElement`. The boundary
split it, and its KerML variant's inputs, the KerML clause alone, differ from the shared
unit's, which had both clauses. Both of its variants are pending.

Bad: the boundary is a hand-maintained list, and reach is only as good as the Tier B′
bodies and the reference extraction. A third leak would show up as a KerML-only production
turning shared. `grammar_plan.py` prints the counts on every run, and a jump in the shared
count is the signal. Retired units now outnumber live ones: every re-scoped unit leaves its
shared predecessor behind as history. `grammar_status.py` therefore reports progress
against live units only.

Not resolved here: whether each boundary deviation is an editorial oversight OMG would
accept. Both entries say what would make them revisit.

## Pros and cons

**Option 1**: simple, and wrong for the whole expression layer. SysML would lose the
expressions it depends on.

**Option 2**: faithful to what each language reaches, and it makes the one judgment involved,
where SysML stops borrowing, explicit and cited instead of implicit.

**Option 3**: no change. It leaves the defect that makes ADR-0014's guarantee hollow.
