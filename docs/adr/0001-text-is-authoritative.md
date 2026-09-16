---
title: "Text is authoritative; diagrams are projections"
status: accepted
date: 2026-09-15
deciders: [David]
---

# Text is authoritative; diagrams are projections

## Context and problem statement

A graphical editor over a text-first language has two surfaces onto one model. Something
has to be the source of truth when they diverge, and files will be edited outside the
tool constantly — by hand, by Git operations, by other SysML tools.

## Decision drivers

- Undo must be coherent across both surfaces
- External edits are routine, not exceptional
- Divergence between the diagram and the file is the failure mode that makes hybrid
  modeling tools feel untrustworthy

## Considered options

1. Text file is authoritative; the diagram is a computed projection
2. A resident model that both surfaces mutate as peers, reconciled to text on save

## Decision outcome

Option 1. The file is the model. The diagram is derived.

### Consequences

Good: one undo stack. External edits need no merge logic — file watcher, reparse, redraw.
The class of bug where the diagram and the file disagree cannot occur.

Bad: an in-progress gesture has no textual form, so an ephemeral pending layer is
required outside the model. Every structural edit pays workspace re-resolution, since
adding a definition in one file changes what resolves in every file that imports it.

Forces ADR-0004 and ADR-0007.

## Pros and cons

**Option 1** — simple invariant, honest for a text-first language; pays reparse cost on
every structural edit.

**Option 2** — cheaper structural edits and easier in-progress state; requires two-way
reconciliation, a second undo stack, and a merge story for external edits.
