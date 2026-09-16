---
title: "Layout and styling live in a split sidecar"
status: accepted
date: 2026-09-15
deciders: [David]
---

# Layout and styling live in a split sidecar

## Context and problem statement

SysML v2 has no diagram interchange format. SysML v1 had DI; v2 dropped it. Geometry and
appearance have nowhere to live in the language, so a sidecar is forced rather than chosen.

## Decision drivers

- Styling is small, shared, and belongs in review
- Positioning is per-element, machine-written, and churns on every drag
- Both must be diffable, but only one will ever be read by a human

## Considered options

1. Two files: a cascading stylesheet and a per-view layout cache
2. One combined sidecar
3. Geometry stored in the model as metadata annotations

## Decision outcome

Option 1.

`notation.style` cascades by metaclass, then stereotype or metadata annotation, then
explicit element reference. Hand-written, committed, reviewed.

`view.layout` is per-view geometry, machine-written, regenerable by auto-layout wherever
a position is not pinned.

### Consequences

Good: the stylesheet becomes a program-wide notation standard with value independent of
this tool. The layout cache can be deleted and rebuilt without losing anything authored.

Bad: two files to keep consistent, and a schema version on each.

Option 3 was rejected because it puts churning machine output into files engineers read
and diff, and because geometry is genuinely not part of the model.

## Pros and cons

**Option 1** — matches the two lifecycles; two files.

**Option 2** — one file; couples a reviewed artifact to one that changes on every drag.

**Option 3** — travels with the model; pollutes source and makes every drag a model edit.
