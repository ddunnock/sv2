---
paths:
  - "vendor/**"
  - "grammars/**"
  - "docs/conformance-target.toml"
  - ".claude/state/grammar/**"
  - ".claude/state/coverage.json"
  - ".claude/state/grammar-diff.json"
  - ".claude/state/deviations.json"
  - "docs/DERIVATION.md"
---

# Grammar, metamodel, and the vendored inputs

Read `docs/DERIVATION.md` before doing anything here. It states precisely what can and
cannot be derived, and the most common mistake in this area is assuming the abstract
syntax can produce the concrete syntax. It cannot.

## Nothing under `vendor/` is editable

Every file is pinned by sha256 and, for OMG artifacts, by File ID. The PreToolUse hook
blocks edits. To move to a new upstream version:

```bash
python3.11 scripts/vendor_sync.py --accept-new     # needs network; a deliberate act
```

then record why in `.claude/state/deviations.json`. An OMG namespace-dated URL changing content is
unusual and worth understanding before accepting.

## Derived artifacts are outputs, not inputs

`.claude/state/grammar/*.json` regenerate byte-identically
from the pinned grammars. If one is wrong, the extractor or the pinned input is wrong —
never the file. Fix upstream and regenerate.

## Rule bodies are written by hand, per production

Take the production name from `.claude/state/grammar/productions.json`. Take the draft
_rule_ and its clause number from `.claude/state/grammar/bnf-productions.json`, derived
from the pinned Tier B′ transcription. Then **retrieve that clause** and check the draft
against it: Tier B′ is non-normative and self-declares manual corrections, so an
unchecked body is a copy, not a derivation. Consult the Xtext as a second opinion.

Where the transcription and the clause disagree, the clause wins and the production is a
`conflict` for the `grammar-adjudicator` agent.

Do not port Xtext rule bodies. The Xtext encodes the pilot parser's LL limitations as if
they were language rules — `PackageBodyElement` is manually inlined to avoid incremental
parser loops, `MultiplicityRange` is narrowed because general bound expressions cause LL
issues, and syntactic predicates (`->`, `=>`) exist purely to resolve ambiguity a
recursive-descent parser does not have. Porting them makes the parser less conformant while
making it look more conformant, because it matches the reference implementation.

## When the Xtext and the specification disagree

`scripts/grammar_diff.py` finds these for you rather than leaving them to be discovered
one broken file at a time. Each difference needs an entry in `.claude/state/deviations.json`, keyed
by `production`, with at least one piece of evidence:

- **Xtext only** — usually a pilot parser workaround. Usually follow the spec.
- **Specification only** — usually unimplemented upstream. Implement it; note that the
  corpus will not exercise it.

Neither resolution is automatic and neither is a defect by itself. An unreviewed one is.

## The two sources are not equally authoritative

The specification is normative; the Xtext is not. But the corpus is what real files
contain, and where the release BNF contradicts the pilot corpus, following the corpus is
often correct. Say which you followed and why. Do not resolve it silently in code.

## Marking productions

```rust
// production: PartDefinition
fn parse_part_definition(p: &mut Parser) { ... }
```

`bnf_coverage.py` reads these. A marker naming a production absent from the inventory
fails the gate. Leaving a production unmarked is fine and honest — it reports as
`unimplemented`.

## Known upstream discrepancy

An open OMG issue reports that the normative `KerML.xmi` contains associations in the
`Connectors` and `Interactions` packages left over from earlier TBD abstract syntax, which
should have been deleted and do not appear in the specification document. The normative
machine-readable artifact and the normative document disagree in at least one place. Treat
neither as infallible.
