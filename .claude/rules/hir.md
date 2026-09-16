---
paths:
  - "crates/sv2-hir/**"
---

# sv2-hir — desugaring and implied specialization

This crate turns concrete syntax into abstract syntax. Two transformations, both large,
both easy to underestimate.

## Desugaring is not a rename

SysML v2 textual notation is heavy sugar. One declaration expands into several reified
elements — a usage, an owning membership, and a feature typing — and the expansion is
normative. Do not invent a flatter shape because it is more convenient to draw from; the
view model downstream is where convenience belongs.

Retrieve the mapping from the wiki navigator per production. Do not infer it from a
neighbouring production that looked similar.

## Implied specializations are invisible in the source

Definitions and usages implicitly specialize library elements that appear nowhere in the
text. A `part def` specializes `Parts::Part`; an `action def` specializes
`Actions::Action`. Omit these and nothing inherits, and the control-flow names every
action body relies on fail to resolve.

Every injection site needs a clause citation in a comment. This is the phase where a
plausible-looking guess does the most damage, because the result still parses and still
resolves — just wrongly.

## The IR admits everything that parsed

Elements with unresolved references, failed constraints, or missing types still enter the
IR, carrying their diagnostics. Filtering on validity here makes the renderer blank out
mid-edit. See ADR-0002.

Validity gates writes — export, emit, and any graphical edit that produces text. It never
gates admission.
