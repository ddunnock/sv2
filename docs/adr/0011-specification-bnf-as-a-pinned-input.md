---
title: "Specification BNF as a pinned input"
status: accepted
date: 2026-09-16
deciders: [David]
---

# Specification BNF as a pinned input

Amends ADR-0010, which remains in force. This adds a tier; it removes nothing.

## Context and problem statement

ADR-0010 recorded that the concrete textual syntax is published in no
machine-readable form, that the normative BNF exists only inside the specification
PDFs, and that "the SysML-v2-Release repository contains no grammar at all."

That last statement was wrong when it was written. `SysML-v2-Release/bnf/` contains
`SysML-textual-bnf.kebnf` (1721 lines) and `KerML-textual-bnf.kebnf` (1467 lines): a
transcription of the specification's textual BNF that carries the specification's own
clause numbers inline as comments, and that declares for each production the metaclass
it constructs and the abstract-syntax assignments that construct it.

At the sha pinned here it yields 641 production definitions against 631 rules in the
Pilot Xtext — the same order of magnitude, from an independent restatement of the same
language.

ADR-0010's plan for rule bodies was therefore to retype ~640 productions out of a PDF.
That is not merely slow: a body typed from a PDF and a body wrong in the PDF-reading
are indistinguishable afterwards, because neither leaves a diff.

## Decision drivers

- Invariant 4: every claim about the language traces to a source. A clause citation
  is the unit of evidence, and nothing about that changes
- A transcription's errors are cheap to find by diffing and expensive to find by
  retyping
- Two independent non-normative restatements that fail differently are worth more
  than either alone — the Xtext errs toward its parser generator, a transcription
  errs toward typos
- No new authority may be created: the PDF is still the only normative statement

## Considered options

1. Leave ADR-0010's plan intact; retrieve every clause by hand from the PDF
2. Pin the transcription and treat it as the grammar
3. Pin the transcription as a draft-and-citation source, with the clause arbitrating

## Decision outcome

Option 3, as **Tier B′**.

`vendor/spec-bnf/*.kebnf`, pinned by the commit sha behind release tag `2026-08`
(`fb97b754f29588b8e9c7a35f370880cd15eb29e7`) rather than by the tag, because a tag can
be moved. `scripts/extract_bnf.py` derives
`.claude/state/grammar/bnf-productions.json`: per production, the transcribed body,
the metaclass, and the clause number printed beside it in the source.

What Tier B′ is used for: the draft rule body, and the spec-side production inventory
that `scripts/grammar_diff.py` differences against the Xtext.

What Tier B′ is **not**: an authority. It is non-normative and self-declares manual
correction — its first line reads `// Manual corrections by HP de Koning`. Where it
and the clause disagree, the clause wins and the disagreement is a `conflict` for the
`grammar-adjudicator` agent.

The evidence standard is unchanged. A unit still cites a clause; what changed is that
the clause is now checked against a written body instead of transcribed into an empty
one. A Tier B′ body that has not been checked against its clause is a copy, not a
derivation, and `spec-conformance-reviewer` treats it as one.

### Consequences

Good: rule bodies start from a written artifact with its citation attached. A second
independent restatement makes `grammar_diff.py` a real differential rather than an
inventory comparison against a name list. The metaclass declarations cross-check the
Xtext-derived metaclass map against a source that is not the Xtext.

Bad: a fourth pin, and a source whose failure mode — a plausible transcription error
in correct-looking notation — is the exact failure this repository is built to
prevent. That is why the clause citation stays mandatory rather than becoming a
formality once a body is already present. It is also a real risk that reviewers will
treat a present body as a checked body; the ledger is what distinguishes them.

Note: ADR-0010's factual claim about the release repository is corrected in place, in
its own Context section, rather than left to be found by someone who believes it.

## Pros and cons

**Option 1** — no new source to audit; spends the project's scarcest resource on
transcription, and produces errors that leave no diff to review.

**Option 2** — fastest; creates a second authority, which is precisely what ADR-0010
refused when it declined to port the Xtext, and would launder a hand-corrected file
into a normative one.

**Option 3** — keeps one authority and gains a draft plus a citation index; costs a
pin and demands the discipline to keep "has a body" separate from "has been checked."
