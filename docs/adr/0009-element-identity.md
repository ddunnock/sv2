---
title: "Element identity strategy"
status: superseded
superseded-by: 0016
date: 2026-09-15
deciders: [David]
---

# Element identity strategy

## Context and problem statement

The sidecar must name elements, and SysML v2 textual notation provides no element
identifiers. The only natural handle is the qualified name, which changes under exactly
the operations an editor exists to perform: rename, move between packages, extract to a
new file.

A layout entry keyed on `Vehicle::engine` survives until someone renames the package, at
which point every entry beneath it orphans silently and the diagram reflows to nothing.

## Decision drivers

- Silent orphaning is the worst outcome; visible failure is recoverable
- Files are authored and edited outside the tool, so identity cannot depend on the tool
  having observed the change
- Source files are read and diffed by engineers, and machine-generated content in them
  has a review cost

## Considered options

1. **Injected IDs** — a UUID on each element as a metadata annotation, so identity is
   real model content
2. **Name-keyed** — the sidecar stores qualified names
3. **Fingerprint reconciliation** — the sidecar stores a structural signature (metaclass,
   parent, typing, neighbor set) and matches on load when a name has moved
4. **Layered** — injected IDs as the durable anchor, name lookup as the fast path,
   fingerprint matching as recovery for files that arrived without IDs

## Decision outcome

**Superseded by [ADR-0016](0016-element-identity-via-petname-notes.md), 2026-09-18.**

This record framed the question and never answered it. ADR-0016 answers it, and answers
it as Option 4 below: a durable anchor written into the file, an exact name match as the
fast path, and fingerprint matching as recovery for files that arrive without one. What
it changes is the carrier. This record assumed the anchor would be an injected UUID in a
metadata annotation; ADR-0016 makes it a petname in a textual **note**, which no
production in either grammar can consume, so it adds no model element and no diagnostic
in another tool. The review-noise objection below is answered by that change rather than
dismissed: what a reviewer sees is `//* @id maple-sunrise-314 */`, not a UUID.

Nothing here is contradicted. Read ADR-0016 for the decision; this record is kept for
the options it rules out and the reasoning that got there.

### What this record said while it was open

**Not yet decided.** Option 4 is the likely answer, since the three are complementary
rather than competing. The real question is narrower: whether injected IDs are acceptable
in source files that go through program review.

### Proposed resolution path

Write UUIDs into one representative model file, put it through a normal review, and see
whether anyone objects. That is cheaper than any amount of further analysis and it
settles the only genuinely open part.

### Consequences of deferring

*Historical: this is what deferring cost while the question was open.*

The sidecar schema currently carries a placeholder (`"identity": "injected-uuid"`). The
parser and resolver are unaffected, so this does not block `sv2-syntax` or `sv2-hir`.
It must be settled before `sv2-resolve` exposes stable element handles, because that API
shape depends on the answer.

## Pros and cons

**Injected IDs** — survive rename, move, and extract; portable to other tools; absent
from hand-authored files, and add machine-generated noise to reviewed source.

**Name-keyed** — trivial to implement, source stays clean; fails on the first refactor,
and fails silently.

**Fingerprint** — recovers from external edits with no source changes; ambiguous when two
siblings are renamed in one operation; a real heuristic to build and tune.

**Layered** — covers every scenario; three mechanisms to maintain and a precedence order
to get right.
