---
paths:
  - "crates/sv2-resolve/**"
---

# sv2-resolve — libraries, names, derived properties, constraints

The largest crate and the one where correctness is hardest to eyeball. Four phases.

## Standard library bootstrap

The libraries are themselves written in KerML and SysML, so the resolver has to resolve
them before it can resolve anything else. Decide and record how deep the bootstrap goes;
at least one prior implementation stopped short of the Kernel Libraries and lets
inherited defaults that need unimplemented kernel functions degrade to null.

That is a defensible choice. It is not a defensible accident. If a default degrades,
it must be recorded as a diagnostic, never silently produce `None`.

Libraries are vendored. No network access at runtime, ever — this has to work air-gapped.

## Name resolution

Implement the visibility rules as specified, not as they seem reasonable:

- imports are private by default
- `public import` re-exports to sibling and child scopes
- `protected import` is visible to children and is not re-exported
- namespace, membership, and recursive (`::*::**`) forms each behave differently

Resolution also traverses subsetting and redefinition chains, and feature chains must
type-check at every step. Retrieve each rule; do not reconstruct it from the general shape.

## Derived properties are a fixpoint, not a traversal

Inherited memberships, effective features, and feature-chain types are mutually recursive
over the model graph. Write it as an explicit fixpoint with cycle detection. A recursive
descent that happens to terminate on the test models will not terminate on a real one.

Cycles in specialization are a diagnostic, not a panic and not a hang.

## Constraints

Each implemented constraint carries the clause identifier it came from:

```rust
// constraint: Type::no_cyclic_specialization
```

A constraint without a citation cannot be reviewed and should not be merged. A constraint
you cannot find in the spec is one you invented.

## Invalidation

Adding a definition in one file changes what resolves in every file that imports it.
Until dependency-tracked invalidation exists, re-resolve the workspace and measure. Do
not build the complex version before the simple one is demonstrably too slow — and when
you do, say so in an ADR.
