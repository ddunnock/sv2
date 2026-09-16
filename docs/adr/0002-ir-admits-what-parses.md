---
title: "The IR admits everything that parses, not only what validates"
status: accepted
date: 2026-09-15
deciders: [David]
---

# The IR admits everything that parses, not only what validates

## Context and problem statement

The DEAL compiler admits only validated constructs into its IR, and renders from the IR.
Carrying that invariant here collides with editing: an editor's file is invalid for most
of the seconds it is open.

## Decision drivers

- A rename leaves references dangling for several seconds during normal typing
- The renderer draws from the IR
- The DEAL invariant is valuable and should be preserved where it actually applies

## Considered options

1. Admit everything that parses; each element carries its own diagnostic state
2. Admit only validated elements, as DEAL does

## Decision outcome

Option 1, with the DEAL guarantee relocated rather than dropped.

Validity gates **writes** — export, emit, and any graphical edit that produces text.
It does not gate **reads**. Rendering decorates by diagnostic state rather than filtering
on it: an unresolved type gets a red underline on its compartment row, not a vanished box.

A file that fails to parse entirely retains its last good subtree, so the diagram holds
still while the user types.

### Consequences

Good: no flicker during normal editing. Diagnostics are locatable in the diagram, which
is where a modeler is looking.

Bad: every consumer of the IR must handle partially-resolved elements. The type system
has to make that unavoidable rather than optional — an unresolved reference should not be
representable as a resolved one.

## Pros and cons

**Option 1** — stable rendering; more states for consumers to handle.

**Option 2** — every IR element is known-good; the diagram blanks and reappears on every
keystroke that touches a name.
