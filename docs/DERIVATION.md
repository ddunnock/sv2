# What can and cannot be derived

Read this before assuming a fact about SysML v2 or KerML can be extracted from the
pinned inputs. The most expensive mistake available in this repository is to assume
that because the abstract syntax is published in machine-readable form, the concrete
syntax can be generated from it. It cannot, and the rest of this document is the
consequences of that.

The pins themselves are in `docs/conformance-target.toml`; the reasoning behind the
three tiers is ADR-0010.

## The asymmetry

OMG publishes almost everything about these languages in normative,
machine-readable form — except the one thing a parser needs most.

| Artifact                                  | Published form              | Tier |
| ----------------------------------------- | --------------------------- | ---- |
| Abstract syntax (metamodel)               | MOF XMI, normative          | A    |
| Abstract syntax as schemas                | JSON Schema, normative      | A    |
| Model interchange format                  | JSON Schema, normative      | A    |
| Standard model libraries                  | KPAR archives of notation   | A    |
| **Concrete textual syntax (the grammar)** | **PDF prose only**          | —    |
| Concrete syntax, transcribed              | `bnf/*.kebnf`, non-normative | B′  |
| Concrete syntax, non-normative            | Pilot Xtext, EPL-2.0        | B    |
| Real files a conformant tool accepts      | Release examples, a model   | C    |

The normative BNF exists only inside the specification documents. Nothing published
by OMG restates it in machine-readable form.

Two non-normative restatements exist, and they fail differently, which is the only
reason it is worth carrying both:

- **Tier B′** — `bnf/SysML-textual-bnf.kebnf` and `bnf/KerML-textual-bnf.kebnf` in
  SysML-v2-Release. A transcription of the BNF that keeps the specification's own
  clause numbers as comments and declares, per production, the metaclass it builds
  and the abstract-syntax assignments that build it. Its header reads
  `// Manual corrections by HP de Koning`: it follows the language, and its errors
  are **transcription** errors.
- **Tier B** — the Pilot Implementation's Xtext. It follows a working parser, and its
  errors are **parser-generator** errors, baked in as though they were language rules.

Neither is normative. The PDF clause arbitrates between them, and
`scripts/grammar_diff.py` reports where they disagree rather than leaving it to be
discovered one broken file at a time.

## Derivable

Everything here is produced by a script from a pinned input, regenerates
byte-identically, and is checked by the gate. Never hand-edit one of these: if the
output is wrong, the extractor or the pinned input is wrong.

| Artifact                                   | Derived from              | By                              |
| ------------------------------------------ | ------------------------- | ------------------------------- |
| `.claude/state/grammar/productions.json`   | Pilot Xtext (Tier B)      | `scripts/extract_productions.py` |
| `.claude/state/grammar/keywords.json`      | Pilot Xtext               | same                            |
| `.claude/state/grammar/metaclass-map.json` | Pilot Xtext + OMG XMI     | same, checked by `check_metaclass_map.py` |
| `.claude/state/coverage.json`              | spec BNF + parser markers | `scripts/bnf_coverage.py`       |
| `.claude/state/grammar/bnf-productions.json` | Spec BNF (Tier B′)      | `scripts/extract_bnf.py`        |
| `.claude/state/grammar-diff.json`          | Xtext inventory vs. spec BNF | `scripts/grammar_diff.py`    |
| `.claude/state/decisions.json`             | ADR frontmatter           | `.claude/scripts/index_decisions.py` |

What this buys: a production **inventory** — the names of the rules, the token set,
and which metaclass each rule constructs. That is enough to drive the lexer, to
measure coverage honestly, and to know what has not been implemented yet.

## Derivable as a draft, never as a claim

**Rule bodies, from Tier B′.** `extract_bnf.py` yields, for each production, the
transcribed body and the clause number printed beside it in the source. That is a
starting draft and a citation to check, not an answer: the transcription is
non-normative and self-declares manual corrections, so a body that arrives from it
has been *proposed*, not *established*.

A unit is established when its clause has been read and agrees with the transcribed
body. Where they disagree the clause wins, and the disagreement is a `conflict` for
the `grammar-adjudicator` agent — the one thing that must never happen is picking
whichever reading is easier to implement. The `derive-grammar` skill is that
workflow; `.claude/rules/derivation.md` is the rule set for a single unit.

What changed by adding Tier B′ is the cost of the check, not the standard. Before, a
body was typed from a PDF; now it is diffed against one. The citation requirement in
invariant 4 is unchanged.

## Not derivable

**Desugaring.** One line of SysML v2 notation expands into several reified abstract
syntax elements — a usage, an owning membership, a feature typing. The expansion is
normative and is stated in the specification text; nothing in the XMI implies it.
Tier B′ records assignments (`ownedRelationship += ...`) per production, which names
the pieces but does not state the normative expansion rule — that is still the clause.

**Implied specializations.** A `part def` specializes `Parts::Part`; an `action def`
specializes `Actions::Action`. These appear nowhere in the source text and nowhere in
the grammar. Every injection site needs a clause citation at the site.

**Name resolution and visibility.** Import forms, re-export behaviour, and the
traversal of subsetting and redefinition chains are specified rules, not properties
of the metamodel graph. `.claude/rules/resolve.md` lists the ones most often guessed
wrong.

**Constraints.** Each one is a named clause in the specification. A constraint you
cannot find in the spec is a constraint you invented; a constraint without its
identifier in a comment cannot be reviewed.

## Why the Xtext is a second opinion and never the source

The Pilot Xtext encodes its own parser generator's limitations as though they were
language rules:

- `PackageBodyElement` is manually inlined to avoid incremental parser loops.
- `MultiplicityRange` is narrowed because general bound expressions cause LL issues.
- `->` and `=>` syntactic predicates exist to resolve ambiguity that a
  recursive-descent parser does not have.

Porting any of these produces a parser that is **less** conformant while looking
**more** conformant, because it agrees with the reference implementation. That
failure is invisible in a positive-only corpus sweep, which is why
`tests/rejection/` exists.

`scripts/grammar_diff.py` reports every place the Xtext inventory and the
specification BNF disagree, so those differences are a generated report rather than a
discovery made months later. Each one needs an entry in
`.claude/state/deviations.json` with evidence:

- **Xtext only** — usually a pilot workaround. Usually follow the spec.
- **Specification only** — usually unimplemented upstream. Implement it, and expect
  the corpus not to exercise it.

Neither resolution is automatic, and neither is a defect by itself. An unreviewed one
is.

## Neither source is infallible

An open OMG issue reports that the normative `KerML.xmi` retains associations in the
`Connectors` and `Interactions` packages left over from earlier TBD abstract syntax,
which do not appear in the specification document. The normative machine-readable
artifact and the normative document disagree in at least one place.

Tier B′ is a transcription and says so on its first line. A production whose body is
wrong there is wrong in a way that reads exactly like a language rule, because it is
written in the same notation as every rule that is right. The clause is what tells
them apart, which is why the clause is cited and not merely consulted.

Where the release BNF contradicts what real files contain, the corpus is often the
better guide — but say which you followed and why. Do not resolve it silently in
code.

## What ties the tiers together

`scripts/check_namespace_link.py` asserts that the metamodel namespace the Xtext
imports equals the namespace of the pinned XMI. That is what detects the Pilot
drifting ahead of the specification, which no per-file hash would catch. The hashes
in `vendor/sources.lock.toml` catch the other failure: content republished at the
same URL.

## Practical consequences

1. A production is implemented from its clause. Tier B′ supplies the draft body and
   names the clause; Tier B is consulted afterwards, as a second opinion. Not the
   other way round, and a Tier B′ body that has not been checked against its clause
   is not a derivation — it is a copy.
2. Isolation is the rule while deriving: one unit, its own context pack, nothing
   else. Order-independence is what makes the result reproducible across sessions and
   across OMG releases, and the unit fingerprint is a claim about exactly that.
3. A conflict between sources stops the work and goes to the `grammar-adjudicator`
   agent. Picking whichever reading is easier to implement is not adjudication.
4. Coverage counts `unimplemented` honestly, against the **specification's** grammar
   rather than the Xtext's 727 rules — every one of the 281 reviewed differences resolved
   `follow_spec`, so the Xtext-only rules are productions this parser has decided not to
   have. The unit is the live **grammar unit** of ADR-0015, 554 of them, not the 558
   inventory names: a production the two languages state differently is two units, and
   counting its name once let a SysML marker report KerML's different production
   implemented too; the names also included the lexical terminals, which the lexer
   reads. A marker on a split production must say `@kerml` or `@sysml`. `absent` is the
   defect, and it separates its causes: a marker naming an Xtext-only production means a
   pilot rule was **ported**, which is the failure this document exists to prevent; one
   on a split production with no scope, or a scope with no live unit, cannot say what it
   claims; and one naming no unit at all was invented or misspelt.
