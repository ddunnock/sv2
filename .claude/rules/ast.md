---
paths:
  - "crates/sv2-ast/**"
---

# sv2-ast — typed accessors over the CST

This crate is a view. It owns no data, and every rule below follows from that.

## Owns no data

An accessor type wraps exactly one `SyntaxNode` and nothing else. It returns nodes and
tokens; the caller asks those for text. No `String`, `Box<str>`, `Vec<String>` field, and
no public function returning `String`. `scripts/check_ast_leaks.py` fails on each.

ADR-0004 justifies the lossless tree partly because this crate makes it affordable.
Copying text out is how that stops being true.

## The marker, and the three states

Every accessor carries `// node: Kind`. `scripts/ast_coverage.py` counts markers against
`.claude/state/node-kinds.json`:

- **implemented** — a marker claims a kind. Counted.
- **unimplemented** — a kind no marker claims. Tracked, never a failure.
- **absent** — a marker claims a kind that does not exist. A defect: the kind is added to
  the parser's `NODES` and regenerated, or the marker is wrong.

A large unimplemented count is the honest state of this crate, not a problem to fix.

## A malformed case per accessor, in the same change

Three things, because absence is not the only way to be wrong:

1. the parse still yields a tree — it always does, and the accessor sees a partial one;
2. the accessor reports **absence** rather than guessing;
3. the neighbouring accessors still answer.

Where the grammar admits one child and the tree carries two, the case pins **which** is
chosen. `.claude/rules/tests.md` governs where expectations may come from; read it.

## What this crate does not do

It raises `ID-MALFORMED` and `ID-ORPHAN`, because attaching an `@id` note to its
declaration needs nothing beyond the current file. It does **not** allocate an identifier,
detect a duplicate, resolve a name, or construct a metaclass. Those need more than one
file, or need the metamodel, and writing one here is how the layering quietly dies.

## Test and development code

Unit tests at the bottom of the file they test, in `#[cfg(test)] mod tests` — the only
place a test reaches a private item, and compiled out of the shipped crate. Everything
else lives outside `src/`: shared support in `tests/trees/`, data in `tests/malformed/`,
tooling in `examples/`. `utils`, `helpers`, `common`, `misc`, `manager` and `handler` are
banned names anywhere under `crates/`, test files included.
