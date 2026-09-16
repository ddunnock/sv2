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
seconds it is open. Two entry points:

- `parse()` — strict. Returns a tree plus the full diagnostic list.
- `parse_for_editor()` — resilient. Always returns a tree, with error nodes where recovery
  kicked in. Never returns `None`, never panics, never loses the parts that did parse.

A resilient parse that drops a whole file because one token was wrong makes the diagram
blank on every keystroke. Recover at the nearest enclosing body and continue.

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
