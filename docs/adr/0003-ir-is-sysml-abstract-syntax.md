---
title: "The IR is SysML v2 abstract syntax; KerML arrives by specialization"
status: accepted
date: 2026-09-15
deciders: [David]
---

# The IR is SysML v2 abstract syntax; KerML arrives by specialization

## Context and problem statement

KerML is sometimes described as SysML v2's intermediate representation. If that were
true, the natural design would lower SysML to KerML and render from the kernel.

## Decision drivers

- The renderer distinguishes part, item, action, port, and state usages by keyword,
  box style, and compartment rules
- Both `.sysml` and `.kerml` files must be readable
- A single semantics layer is cheaper to build and verify than two

## Considered options

1. SysML v2 abstract syntax as the IR, with KerML inherited by specialization
2. Lower SysML to KerML and treat the kernel as the IR

## Decision outcome

Option 1.

SysML's abstract syntax _specializes_ KerML's rather than reducing to it — a `PartUsage`
does not become a `Feature`, it is-a `Feature`. Nothing is discarded, so there is no
lowering step. An IR earns its name by throwing information away; this relationship
throws nothing away, which is exactly why it is not one.

Lowering would flatten the usage kinds to `Feature` and destroy the distinctions the
renderer exists to show.

### Consequences

Good: two concrete-syntax front ends over one metamodel and one renderer. The
"two languages" problem is smaller than it appears — one semantics layer, two parsers.

Bad: the spec's reified memberships and relationship elements are an awkward shape to lay
out from, so a view-model projection sits above the IR. That is a view model, not a
second IR, and it must stay derived.

Note: KerML textual notation re-serialization is a separate concern from the metamodel
and is out of scope here.

## Pros and cons

**Option 1** — preserves every distinction; one metamodel to verify against the spec.

**Option 2** — smaller case analysis downstream; loses the information the renderer needs,
and misreads specialization as reduction.
