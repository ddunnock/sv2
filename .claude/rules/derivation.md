---
paths:
  - ".claude/state/grammar/units/**"
  - ".claude/state/grammar/reference.json"
  - ".claude/scripts/grammar_*.py"
  - ".claude/scripts/grammar-preflight.sh"
  - ".claude/scripts/_grammar.py"
  - ".claude/scripts/_earley.py"
---

# Grammar derivation

Full workflow in the `derive-grammar` skill. The rules that matter while editing a unit:

## One unit, its own inputs, nothing else

A unit is derived from its context pack alone — its clause, its Xtext rule, its corpus
instances. Do not read other unit files, do not consult the assembled grammar, do not carry
a pattern from the previous unit.

This is not a style preference. Isolation is what makes the result independent of
derivation order, and order-independence is the only reason an AI-assisted derivation is
repeatable across sessions and across releases. A unit derived with broader context has a
fingerprint that no longer describes what produced it.

## Two languages: shared units and variants

KerML and SysML are two grammars sharing a vocabulary (ADR-0014). A production both state
the same way is one shared unit, `Name.json`, and belongs to both. A production they state
differently is two variants, `Name@kerml.json` and `Name@sysml.json`, each carrying
`"scope"`. The plan decides which, deterministically; derivation never splits or merges a
unit, and never argues that one language's body stands in for the other's.

A variant is derived from its own language's clause only, which is all its pack contains.
A shared unit's references must resolve in both grammars, and the checker enforces it.

## The specification is the source; the Xtext is a second opinion

Write the rule from `spec_clause_text`. Consult `xtext_rule_text` to check yourself.

Never port an Xtext LL workaround: manually inlined bodies, ranges narrowed because general
expressions cause parser-generator problems, `->` and `=>` predicates. They exist to satisfy
a tool this project does not use, and porting them makes the grammar describe the pilot
implementation rather than the language.

## Conflicts stop, they do not get resolved quietly

If the sources genuinely disagree, set `status: "conflict"`, record both readings in
`notes`, and run the `grammar-adjudicator` agent. Choosing whichever is easier to implement
is not adjudication.

## Evidence must be checkable

At least one entry, and each must resolve: a clause reference that exists, a corpus file
that actually contains an instance. Evidence that merely exists is not evidence.

## Fix the rule, never the check

`grammar_check_unit.py` is deterministic. If it fails, the rule is wrong. Do not widen the
allowed keyword set, do not add a reference to a production you have not derived, and do not
edit the checker.
