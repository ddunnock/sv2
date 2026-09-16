---
name: add-production
description: Implement one grammar production end to end — parser, CST node, snapshot, positive and negative tests, coverage marker. Use when adding support for a new piece of SysML v2 or KerML syntax, or when the user says "add support for", "implement the production", "handle this syntax", or names a production from the grammar.
---

# Add a production

One production per invocation. If the request names several, do the first and say what
remains; batching is how partially-implemented productions get marked complete.

## 1. Establish the production exists

```bash
grep -n '^<NAME>' grammars/*.g4
```

Not present? Stop. Either the name is wrong or the production is not in the pinned
grammar, and inventing it is the exact failure this workflow prevents.

## 2. Retrieve the semantics before writing any code

Use `sysml-v2-wiki-navigator` or `kerml-wiki-navigator` to get:

- the target metaclass and its supertype chain
- which constraints govern it
- what the production desugars into — how many reified elements, and of what kinds
- whether it carries an implied specialization

Write these into the working notes before implementing. Every one of them becomes a
citation in the code.

## 3. Find a real example

Search the corpus for files using this production. A real instance beats a constructed
one, because constructed examples tend to exercise only the shape you already imagined.

```bash
grep -rl '<keyword>' "${SV2_CORPUS_DIR:-tests/corpus}" | head
```

## 4. Write the failing tests first

- **positive** — the corpus example, or a minimal case citing its clause
- **negative** — into `tests/rejection/`, one file per rule the production can violate,
  each with the clause in a header comment
- **round-trip** — the input must survive `parse().text()` byte for byte

Run them. They must fail for the right reason before you proceed.

## 5. Implement

Parser function in `sv2-syntax` with the coverage marker:

```rust
// production: <name>
fn parse_<name>(p: &mut Parser) { ... }
```

Then the CST node kind, the `sv2-ast` accessor, and — if the production desugars or
carries an implied specialization — the `sv2-hir` handling with clause citations at each
injection site.

## 6. Verify

```bash
cargo insta review          # one snapshot at a time, each explained
python3.11 scripts/bnf_coverage.py   # refresh the report
./scripts/gate.sh           # must be green
```

## 7. If it does not fit

If the production cannot be implemented without contradicting an ADR, or the grammar
rejects the specification's own example, **stop and report**. Do not patch the grammar
inside this workflow — that is a separate, evidenced change under `.claude/rules/grammar.md`.
