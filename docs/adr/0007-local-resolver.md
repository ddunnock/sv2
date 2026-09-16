---
title: "The resolver is local and owned"
status: accepted
date: 2026-09-15
deciders: [David]
---

# The resolver is local and owned

## Context and problem statement

Phases 4 through 7 — library loading, name resolution, derived properties, constraint
validation — are the largest part of the build. A mature Java implementation already
exists and exposes results over the Systems Modeling API.

## Decision drivers

- Refusing an illegal gesture must happen before the user releases the mouse
- Air-gapped and CUI environments must be supported
- This is the single largest line item in the project

## Considered options

1. Implement phases 4–7 in-process
2. Source resolved semantics from the pilot implementation over the Systems Modeling API

## Decision outcome

Option 1, and it follows from ADR-0001 rather than being chosen freely.

Classifying an edit — clean, ambiguous, refused, layout-only — requires resolved semantics
at interactive latency. A network round trip per gesture cannot provide that, and an
air-gapped deployment may have no server at all.

### Consequences

Good: interactive classification is possible. No runtime network dependency. The tool is
authoritative about its own semantics, which matters if DEAL normalizes against it.

Bad: this is the expensive decision. It requires the standard-library bootstrap, the full
visibility rules, and a fixpoint over derived properties — and it means owning correctness
rather than borrowing it.

## Pros and cons

**Option 1** — interactive, offline, authoritative; large and correctness-critical.

**Option 2** — far less code, borrows a mature implementation; cannot meet the latency
requirement, cannot run air-gapped, and makes the tool non-authoritative.
