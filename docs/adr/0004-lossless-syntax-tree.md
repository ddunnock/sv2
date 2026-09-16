---
title: "Lossless incremental syntax tree, not an AST"
status: accepted
date: 2026-09-15
deciders: [David]
---

# Lossless incremental syntax tree, not an AST

## Context and problem statement

ADR-0001 routes every graphical edit back through the text buffer. The parse product has
to support producing a minimal textual delta from a model-level change.

## Decision drivers

- Diffs must stay reviewable after a graphical edit
- Comments and formatting are authored content, not noise
- Typing latency and the edit round trip both depend on reparse cost

## Considered options

1. Lossless concrete syntax tree, incremental, with a typed accessor layer above it
2. Parse to an AST and pretty-print on write

## Decision outcome

Option 1. Every byte is recoverable from the tree; `root.text()` equals the input exactly.

Prior art: the Rust implementation in this space builds on `rowan`, the lossless-tree
library from rust-analyzer. That is not incidental — it is what interactive editing
requires, and it is worth copying rather than rediscovering.

### Consequences

Good: a graphical edit changes only the bytes it must. Error recovery is natural, since
an error node is just another node. Incremental reparse is possible.

Bad: more nodes and more memory than an AST. Consumers need the `sv2-ast` accessor layer
to avoid touching trivia. Snapshot tests become mandatory, because tree shape is now
something that can silently drift.

## Pros and cons

**Option 1** — minimal deltas, formatting preserved, recovery-friendly; heavier tree.

**Option 2** — simpler and smaller; reformats the file on every graphical edit, making
diffs unreviewable and destroying comments.
