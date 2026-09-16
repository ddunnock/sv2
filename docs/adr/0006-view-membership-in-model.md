---
title: "Diagram membership stays in the model"
status: accepted
date: 2026-09-15
deciders: [David]
---

# Diagram membership stays in the model

## Context and problem statement

Which elements appear on a diagram, and why, has to be recorded somewhere. It could go in
the sidecar alongside geometry, or in the model itself.

## Decision drivers

- Portability to Cameo, SysON, and the pilot tools
- SysML v2 already provides constructs for this
- Losing the sidecar should not lose the diagram

## Considered options

1. Membership in the model via `view`, `viewpoint`, `expose`, `render`
2. Membership in the sidecar with the geometry

## Decision outcome

Option 1. Membership is expressible in standard SysML v2, so it is portable: a diagram
opened in another conformant tool shows the right elements, laid out differently.

Only geometry and appearance are proprietary.

### Consequences

Good: diagrams survive the tool. A view is reviewable as model content, and viewpoints
can express _why_ a diagram contains what it does rather than just listing members.

Bad: adding an element to a diagram is a model edit and therefore goes through the full
round trip of ADR-0001, unlike moving one.

**Test for violation:** if deleting the sidecar loses which elements are on a diagram,
this decision has been broken somewhere.

## Pros and cons

**Option 1** — portable, reviewable, standard; membership changes cost a reparse.

**Option 2** — cheaper edits; diagrams become meaningless outside this tool.
