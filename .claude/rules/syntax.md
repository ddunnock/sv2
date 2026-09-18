---
paths:
  - "crates/sv2-syntax/**"
  - "crates/sv2-ast/**"
---

# sv2-syntax and sv2-ast — the lossless layer

## The parse tree is lossless. This is not negotiable.

Every byte of the source must be recoverable from the tree: whitespace, line endings,
both comment forms, and the author's spacing choices. A graphical edit downstream has to
produce a minimal text delta, and it cannot if the tree has thrown formatting away.

Concretely:

- Trivia is attached to the tree, never skipped in the lexer.
- `node.text()` for the root must equal the input file exactly. There is a property test
  for this; it is the most important test in the workspace.
- No production may normalize its input — not casing, not spacing, not quote style.

## Error recovery is a first-class path, not a fallback

The editor parses invalid text constantly, because a file is invalid for most of the
seconds it is open. There is **one** entry point and it is always resilient:

```rust
pub fn parse(source: &str, language: Language) -> Parse
```

It always returns a tree, with `Error` nodes where recovery kicked in. It never returns
`None`, never panics, and never loses the parts that did parse. There is no strict
variant, and adding one would be a second parser: acceptance is "the parser reported
nothing", which is `Parse::errors().is_empty()`, not a different function.

`language` is required and has no default. KerML and SysML are two grammars with two
start symbols (ADR-0014), and reading a file against the one its author did not write it
in accepts constructs that language does not have.

A resilient parse that drops a whole file because one token was wrong makes the diagram
blank on every keystroke. Recover at the nearest enclosing body and continue.

## Diagnostics carry a range, never just a sentence

`Parse::errors()` returns `&[Diagnostic]`, and a `Diagnostic` cannot be constructed
without a `TextRange`. That is deliberate: an editor underlines a span, and ADR-0002
decorates a diagram row rather than dropping it — neither can be done with prose.

- The code (`DiagnosticCode`) is the stable identity; the message is free to be reworded.
- Codes are namespaced by the crate that raises them: `PARSE-*` here, `HIR-*` above.
- Severity is a property of the code, not of the site, so two raisers of one code cannot
  disagree about how much it matters.
- `sv2-cli`'s `ErrorCode` is a **different** vocabulary — how the *process* failed, one
  per exit status (STD-002-RS §3.2). Do not merge them.

`OffsetMap` is the single designated boundary for turning a byte offset into anything
else — a line and column now, UTF-16 code units when a language server needs them
(ADR-0013 RISK-013-2). A second conversion anywhere defeats the test that this one is
right.

## sv2-ast owns no data

The AST layer is typed accessors over CST nodes. It holds references and offsets, not
owned strings or cloned subtrees. If you find yourself copying text out of the CST into
an AST struct, the abstraction has leaked — the consumer should hold the node instead.

## Incrementality

Reparse cost must scale with the edit, not the file. Keep node construction free of
whole-file scans and avoid any pass that walks the tree to compute something a node
could carry.

## Snapshots

Tree shape changes are reviewed as snapshot diffs (`cargo insta review`). Accept them
one at a time and only when you can state which code change produced each one. Bulk
acceptance defeats the entire mechanism.
