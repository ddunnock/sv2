---
title: "KerML and SysML are two grammars sharing a vocabulary"
status: accepted
date: 2026-09-17
deciders: [David]
---

# KerML and SysML are two grammars sharing a vocabulary

Builds on ADR-0010 and ADR-0003, both of which remain in force.

## Context and problem statement

The derivation pipeline was built for one grammar: one unit per production name, one
start symbol, and an oracle that read `.kerml` and `.sysml` files against that single
grammar. That held for as long as every production stated in both languages was stated
the same way, and it stopped holding at the start symbol:

```text
KerML 8.2.3.4.1   RootNamespace = NamespaceBodyElement*
SysML 8.2.2.5.1   RootNamespace = PackageBodyElement*
```

Both are normative, each for its own language. A `.kerml` file holds `classifier` and
`feature`; a `.sysml` file holds `part def`. Neither body is a defect, so there is
nothing to adjudicate: the languages differ.

It is not a single production. The Tier B′ inventory states 83 production names in both
language files. Ignoring whitespace and assignments, which change where a value lands
but not what parses, 27 of them have different bodies. One, `RESERVED_KEYWORD`, is lexical
and is not derived, which leaves 26. The difference is often a real difference in what
parses: `TypedBy` is `TYPED_BY OwnedFeatureTyping` in KerML and `DEFINED_BY FeatureTyping`
in SysML, and `MultiplicityRange` is a `multiplicity` declaration in one and a bracketed
range in the other.

ADR-0010 already records the structural fact behind this. At the grammar level
`SysML.xtext` extends `KerMLExpressions`, not `KerML.xtext`: the two grammars are siblings
that share the expression layer, not one grammar that the other extends. ADR-0003 is
about the metamodel, where SysML genuinely specializes KerML. The two relationships are
different and must not be conflated.

## Decision drivers

- ADR-0003: both `.sysml` and `.kerml` files must be readable
- Invariant 4: every claim about the language traces to a source, and a rule derived
  for one language's clause is not evidence about the other's
- The oracle must be able to fail. A grammar that accepts a SysML construct in a KerML
  file passes a positive-only corpus sweep, so that failure is silent
- Units already verified are reviewed decisions and must not be discarded to make the
  design simpler, nor carried across to inputs they were not derived from

## Considered options

1. **Per-language grammars.** Units stay shared where the two languages agree, and split
   into one variant per language where they differ. Two start symbols, one oracle per
   language.
2. **SysML only.** Derive SysML's body wherever the two diverge, and stop reading
   `.kerml` files.
3. **A union grammar.** One `RootNamespace = ( NamespaceBodyElement | PackageBodyElement )*`,
   and likewise wherever the languages diverge.

## Decision outcome

Option 1.

**Which productions split is decided by the plan, not by derivation.**
`divergent_productions` in `_grammar.py` compares the Tier B′ bodies from the two
language files, ignoring whitespace and assignments. It is deterministic, so the same
inputs always produce the same set. A body that differs only in how it is factored still
splits. Judging two factorings equivalent is derivation work, and a variant costs little.

**A variant is a unit with a `scope`.** `RootNamespace@kerml.json` has
`"production": "RootNamespace"` and `"scope": "kerml"`. A shared unit has no scope and
belongs to both grammars. The key is derived from the unit's content, and `load_units`
refuses any file whose name disagrees with its content, so a variant cannot be saved over
the shared unit or shadow it by accident.

**Each language reads its own view.** `grammar_view(units, scope)` returns the live shared
units with that language's variants in their place, keyed by production name, which is
what rules reference. Consistency, reachability and the oracle each run once per language.
The oracle picks the grammar by file suffix. A variant's context pack carries only its own
language's clause, its own language's Xtext file (SysML variants never get `KerML.xtext`),
and its own language's corpus files.

**A shared unit must hold in both grammars.** `grammar_check_unit.py` resolves a shared
unit's references in both views and fails if either one leaves a reference dangling.

**Splitting retires the shared unit.** When a production starts to diverge, the plan
retires its shared unit and creates both variants as pending. When it stops diverging, the
plan revives the shared unit. A live shared unit beside a live variant fails
`grammar_consistency.py` and blocks a freeze.

**Freezing needs both languages.** An oracle run that skipped a language's files blocks a
freeze, because those files were never tested.

### Consequences

Good: the grammar says what each specification says, for each language, and the oracle can
now detect a construct from one language appearing in the other's file. Where the languages
agree, which is most of the grammar, nothing is duplicated.

Bad: four verified units were stated differently in the two languages and were split, so
their verified status is gone: `FeatureChainMember`, `OwnedFeatureChain`,
`OwnedFeatureChainMember` and `ResultExpressionMember`. On inspection, each accepts the
same text in both languages, and the difference is only factoring. They are re-derived per
language anyway, because asserting that equivalence would be a judgment made outside
derivation, and recording it as a carried-forward verification would misstate what was
checked. The count of units to derive grows by the number of splits.

Not resolved here: the Rust parser in `sv2-syntax` implements SysML's `PackageBodyElement`
as its root and applies it to both file kinds. It will need the same split. That is parser
work, and this record only makes it visible.

## Pros and cons

**Option 1**: faithful to both specifications, and the oracle stays meaningful. It costs a
pipeline change and re-derivation of the split units.

**Option 2**: the cheapest honest option. It contradicts ADR-0003's requirement that
`.kerml` files be readable, so that record would have to be superseded, and it throws away
the KerML corpus as evidence.

**Option 3**: needs no pipeline change. It accepts text that neither specification accepts,
and the one check that could catch that, the oracle over a positive corpus, cannot, so the
grammar would be wrong in a way nothing reports.
