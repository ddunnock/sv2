---
name: add-production
description: Implement one grammar production end to end — parser, CST node, snapshot, positive and negative tests, coverage marker. Use when adding support for a new piece of SysML v2 or KerML syntax, or when the user says "add support for", "implement the production", "handle this syntax", or names a production from the grammar.
---

# Add a production

One production per invocation. If the request names several, do the first and say what
remains; batching is how partially-implemented productions get marked complete.

## 1. Establish the production exists, and that it should

The pinned grammar is `vendor/spec-bnf/*.kebnf` (the specification BNF) and
`vendor/pilot/*.xtext` (the Pilot). There are no `.g4` files. The derived inventory is
`.claude/state/grammar/bnf-productions.json`, whose 558 unique names are the denominator
the coverage gate measures against.

```bash
jq -r --arg n Import '.rules[] | select(.name==$n)
  | "\(.file):\(.line)  clause \(.clause) — \(.clause_title)\n\(.body)"' \
  .claude/state/grammar/bnf-productions.json
```

That is existence, both source files, the clause, and the verbatim body in one answer.
Nothing? Stop. Either the name is wrong or the production is not in the pinned grammar,
and inventing it is the exact failure this workflow prevents.

**Then ask whether it is one this repository has decided not to implement.**

```bash
jq -r --arg n ImportPrefix '.deviations[] | select(.production==$n)
  | "\(.bucket): \(.decision)\n\(.rationale)"' .claude/state/deviations.json
```

An `xtext_only` rule with `follow_spec` is a Pilot parser workaround, not a language
construct: build what it constructs inside the rule that uses it, and add no production.
Marking one fails the coverage gate, because it is not in the inventory. `ImportPrefix`,
`ImportedMembership` and `FilterPackageMembershipImport` are all in this bucket — reading
the Xtext alone would have you implement three productions the language does not have.

Already done? `jq -r --arg n Import '.unimplemented_productions | index($n)'
.claude/state/coverage.json` — a null means it is implemented or absent, not pending.

These are `jq` because the PreToolUse hook blocks Bash that names a machine-owned path
unless the command is a known reader. `python3.11 -c` reading one of these files is
denied, and correctly so: the hook cannot tell a read from a write.

## 2. Retrieve the semantics before writing any code

Use `sysml-v2-wiki-navigator` or `kerml-wiki-navigator` to get:

- the target metaclass and its supertype chain
- which constraints govern it
- what the production desugars into — how many reified elements, and of what kinds
- whether it carries an implied specialization

Write these into the working notes before implementing. Every one of them becomes a
citation in the code.

If the navigator's `closure.py` is not installed, the wiki is still readable: the atoms
are markdown under `<wiki>/atoms/production/<Name>.md`, carrying the verbatim grammar and
a `region_sha256`, and `<wiki>/graph.sqlite` has `atoms(id, kind, clause, printed_page,
path, region_sha256)` to resolve a name to its file. Resolve by id, never grep prose.
Confirm the receipt is pinned before citing it:

```bash
grep -c '<region_sha256>' .claude/state/wiki-receipts.json
```

## 3. Find a real example

Search the corpus for files using this production. A real instance beats a constructed
one, because constructed examples tend to exercise only the shape you already imagined.
The pinned corpus is `vendor/corpus` — `tests/corpus` does not exist yet, and
`SV2_CORPUS_DIR` names the sweep's positive corpus, which is a different thing.

```bash
grep -rl '<keyword>' vendor/corpus --include='*.sysml' --include='*.kerml' | head
```

**Count, do not eyeball.** A pattern that begins with a space or `\b` matches inside a
longer form: `[[:space:]]import` matches the space in `private import`, which makes 741
qualified imports look like bare ones and turns a settled grammar into an apparent
spec-versus-corpus conflict. Before concluding the corpus disagrees with the grammar,
count both sides and read the outliers — the four that survived that search were all
comments.

## 4. Write the failing tests first

- **positive** — the corpus example, or a minimal case citing its clause
- **negative** — into `tests/rejection/`, one file per rule the production can violate,
  each with the clause in a header comment
- **round-trip** — the input must survive `parse().text()` byte for byte

Run them. They must fail for the right reason before you proceed.

`scripts/corpus-sweep.sh` runs `tests/rejection/` on every gate, independently of whether
a positive corpus exists, so a case added here is live immediately. A rejection the parser
only makes because the production is unimplemented is honest, but say so in the header and
say what removes it — it is a rejection by absence, not by rule.

## 5. Implement

**The node kinds come first, and they are authored, not derived.** `SyntaxKind` is
generated, so `crates/sv2-syntax/src/generated/` is machine-owned and the hook blocks
writing it. Add the variant to `NODES` in `scripts/gen_syntax_kinds.py`, one line per
node with its production and clause, then regenerate:

```bash
python3.11 scripts/gen_syntax_kinds.py
```

Then the parser. Productions are methods on `Parser` in `crates/sv2-syntax/src/parser.rs`,
each carrying the coverage marker the gate reads:

```rust
// production: Import
fn import(&mut self) {
    self.eat_trivia();
    self.start_node(SyntaxKind::Import);
    // ...
    self.finish_node();
}
```

Mark only what you implemented. A production whose alternatives are partly done is better
left unmarked with the gap recorded in `state.json` than marked and counted — coverage is
a claim, and `unimplemented` is a tracked state while a false `implemented` is a defect.

Two alternatives that share a prefix cannot be chosen until after it. Open the node
retroactively with `builder.checkpoint()` and `start_node_at` rather than guessing and
repairing, and reach for `peek_nth` when one token is not enough to tell them apart.

`sv2-ast` and `sv2-hir` do not exist yet. When they do, the typed accessor and any
desugaring or implied specialization belong there, with the clause cited at each
injection site.

## 6. Verify

```bash
python3.11 scripts/gen_syntax_kinds.py --check   # the generated module is current
python3.11 scripts/bnf_coverage.py               # refresh the report
./scripts/corpus-sweep.sh                        # every rejection case still caught
./scripts/gate.sh                                # must be green
```

`cargo insta review` reviews snapshots one at a time, each explained — never
`cargo insta accept`. It needs a terminal, so where one is not available do not take the
snapshot at all: assert the tree shape structurally, reading the expected nodes off the
productions. An expectation written from what the parser printed proves only that the
parser does what it does.

Prove a new rejection case can fail. Planting a file that parses and watching the sweep
go red takes one minute and is the difference between a harness and a decoration.

## 7. If it does not fit

If the production cannot be implemented without contradicting an ADR, or the grammar
rejects the specification's own example, **stop and report**. Do not patch the grammar
inside this workflow — that is a separate, evidenced change under `.claude/rules/grammar.md`.

Where the specification BNF, the Xtext and the corpus genuinely disagree about one
production, that is the `grammar-adjudicator` agent's job. Check your own evidence first:
the last apparent conflict was a regex artifact, and escalating it would have spent an
agent proving the grammar was right all along.
