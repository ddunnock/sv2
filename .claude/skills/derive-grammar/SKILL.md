---
name: derive-grammar
description: Derive the SysML v2 / KerML reference grammar from the pinned inputs, one production at a time, through six phases. Use when deriving or re-deriving grammar productions, after re-pinning to a new OMG release, or when the user mentions grammar derivation, rebasing the grammar, deriving productions, or the reference grammar.
---

# Derive the reference grammar

Six phases. Only phase 3 uses judgment; the other five are deterministic. That ratio is
the design — it is what makes an AI-assisted derivation repeatable across sessions and
across OMG releases.

Run `python3.11 .claude/scripts/grammar_status.py` first. If units already exist, you are resuming, not
starting: go to whichever phase the status implies.

## Phase 1 — preflight

```bash
.claude/scripts/grammar-preflight.sh
```

Red? Stop. Derivation is the expensive phase, and starting it against a drifted pin or a
stale inventory means throwing the work away. Fix the inputs first.

If it reports no specification BNF clauses, that is a hard stop, not a warning. Deriving
from the Xtext alone produces a grammar that describes the pilot parser rather than the
language. See `docs/DERIVATION.md`.

## Phase 2 — plan

```bash
export SV2_WIKI_CLAUSES=~/.sv2-derivation/bnf-clauses.json   # from export_wiki_clauses.py
python3.11 .claude/scripts/grammar_plan.py
```

It refuses to run without the clause export: every fingerprint hashes the clause text,
so a plan without it would send every verified unit back to pending.

Creates one unit per production with an input fingerprint, and reclassifies existing units
whose inputs have moved. Never edit a unit file to change the plan; change the inputs and
re-run.

A production KerML and SysML state differently becomes two units, `Name@kerml` and
`Name@sysml` (ADR-0014). One that only one language's grammar reaches is that language's
variant alone (ADR-0015); the plan prints how many of each. Name the variant when asking for a pack or checking one:
`grammar_next.py RootNamespace@sysml`. Phases 4 and 5 run once per language.

## Phase 3 — derive, one unit at a time

This is the only phase where you make decisions. Loop:

```bash
python3.11 .claude/scripts/grammar_next.py           # emits the context pack for ONE unit
```

**Work only from that pack.** Do not open other unit files, do not look at the assembled
grammar, do not carry assumptions from the previous unit. Isolation is what makes the
result reproducible: same inputs, same rule, regardless of order or where the session
ended. A unit derived with the whole grammar in view is not reproducible and the
fingerprint on it becomes a lie.

For each unit:

1. Read `spec_clause_text`. This is the source. Write the rule from it.
2. Read `xtext_rule_text` as a second opinion only. If it differs, ask _why_ before
   assuming the spec is wrong.
3. **Never port an Xtext LL workaround.** Manually inlined bodies, ranges narrowed because
   general expressions "cause LL parsing issues", `->` and `=>` syntactic predicates — all
   exist to satisfy a parser generator this project does not use. Porting them makes the
   grammar describe the pilot implementation instead of the language.
4. Check `corpus_instances`. A rule that cannot parse a real corpus line is wrong even if
   it matches the clause you read — you have probably misread the clause.
5. Write `rule` as a JSON AST, plus `decision`, `evidence` (at least one), and
   `status: "derived"`. Write nothing else, and no other file.

**If the sources genuinely disagree**, set `status: "conflict"`, record both readings in
`notes`, and stop. Run the `grammar-adjudicator` agent. Do not resolve a conflict by
picking whichever is easier to implement.

Then check it:

```bash
python3.11 .claude/scripts/grammar_check_unit.py <Production>
```

All checks pass → status becomes `verified` automatically. Any fail → fix the rule, not
the check. Repeat until `grammar_next.py` reports no pending units.

Batch of 10 to 20 units per session is reasonable. There is no penalty for stopping — the
unit files are the checkpoint.

## Phase 4 — consistency

```bash
python3.11 .claude/scripts/grammar_consistency.py
```

Cross-unit structure: undefined references, unreachable productions, normalization. Some
undefined references are expected mid-derivation; they must be empty before freeze.

## Phase 5 — validate against the oracle

```bash
python3.11 .claude/scripts/grammar_validate.py
```

Runs an independent Earley recognizer over the derived grammar against the corpus. A
`MISSED` file names the exact token where the parse stopped — start there, and fix the
production that token belongs to.

`LEAKED` means the grammar accepts something it must reject. That is the more serious
direction and the easier one to overlook, because a grammar that accepts everything passes
a positive-only sweep.

## Phase 6 — freeze

```bash
python3.11 .claude/scripts/grammar_freeze.py
```

Refuses unless every unit is verified and the oracle is clean. Writes `reference.json`,
its sha256, a rendered `reference.ebnf` for human review, and an append-only ledger entry.

## Rebasing onto a new OMG release

This is the reason the whole workflow is fingerprinted.

```bash
# re-pin Tier A / Tier B in docs/conformance-target.toml, then:
python3.11 scripts/vendor_sync.py --accept-new
python3.11 scripts/extract_productions.py
python3.11 .claude/scripts/export_wiki_clauses.py
SV2_WIKI_CLAUSES=~/.sv2-derivation/bnf-clauses.json python3.11 .claude/scripts/grammar_plan.py
python3.11 .claude/scripts/grammar_rebase.py
```

The rebase report tells you exactly which units need work. Unchanged units carry forward
untouched with their evidence intact — do not re-derive them "to be safe", because that
throws away reviewed decisions and reintroduces variance the fingerprint exists to
prevent. Then resume at phase 3 for the delta only.
