---
title: "ADR-0020: Element identity is allocated when a workspace opens"
status: "accepted"
date: 2026-09-18
accepted: 2026-09-18
version: "0.1.0"
decision-makers: David Dunnock
consulted: TBD
informed: TBD
---

# ADR-0020: Element identity is allocated when a workspace opens

## Context and Problem Statement

[ADR-0016](0016-element-identity-via-petname-notes.md) settled what an element
identity *is* — a petname carried in an inline note — and left open *when* one comes
into existence. Its "Allocation" step 3 reads:

> Write the ID note when the element is created in the editor, or when an unidentified
> element is first resolved and reconciliation finds no match.

"First resolved" is the ambiguous half. Resolution is what happens when a workspace is
opened, so the sentence admits two readings that differ in what opening a model *does*:

- **Eager.** Opening resolves every element, finds no match for any of them, and writes
  an ID for each. Opening is a write.
- **Lazy.** An ID is written only when something needs a durable handle — creating an
  element, or moving a diagram node, whose layout record keys off it
  ([ADR-0017](0017-view-scoped-json-lines-sidecar-for-diagram-layout-and-styling.md)).
  Opening is a read.

Nothing in ADR-0016 chooses, and the difference is not cosmetic. **No file in the
corpus carries an `@id` note** — 311 of 311 — so under the eager reading, opening any
existing model rewrites every file in it before the user has typed anything.

The webview needs the answer before its first schema is written. It decides whether
"this element has no durable identity" is a state that exists only while a workspace
loads, or one that every consumer of every element carries for ever.

## Decision Drivers

- **DD-1 — One identity per element, available to everything at once.** The layout
  sidecar (ADR-0017), diagram node keys (STD-004-TS §8.5 rule 4), cross-window
  selection, and the log-safe handle that §9.3 requires instead of a qualified name
  all key off the ID. A model in which some elements have one and some do not makes
  every one of those conditional on a state that has nothing to do with them.
- **DD-2 — Partial identity would be a second admission rule.** ADR-0002 already
  obliges every consumer to handle an element that is admitted but not fully resolved.
  Adding "and may also have no identity" multiplies the states each consumer must
  carry, and the two are independent, so it really is a multiplication.
- **DD-3 — Reconciliation needs a complete index.** ADR-0016 allocation step 2 rejects
  a candidate the resolver's workspace-wide ID index already holds. An index built
  over a workspace where most elements have no ID cannot detect a collision against
  the ones that do not, so lazy allocation weakens the duplicate detection that makes
  random petnames safe.
- **DD-4 — Writing to a person's files on open is not a side effect to hide.**
  Whichever way this goes, the behaviour has to be visible. That is a cost of the
  eager reading, not an argument against stating it.
- **DD-5 — Fewer states, one maintainer.** Every permanent optional carried through
  the contract, the model layer, and every component is sustainment cost.

## Considered Options

- **Option 1 — Eager: allocate for the whole workspace when it opens.**
- **Option 2 — Lazy: allocate when a durable handle is first needed.**
- **Option 3 — Per file: allocate for a file the first time anything in it changes.**

## Decision Outcome

Chosen option: **Option 1, eager allocation on open.**

It is the literal reading of ADR-0016 step 3, and it is the only option under which
DD-1 and DD-3 hold by construction rather than by every consumer's discipline. The
webview's `ElementHandle` therefore keeps an `unidentified` arm for the window between
parse and allocation, and that arm is **transient**: it does not appear in a loaded
workspace, and no component is written to render it as a steady state.

### Consequences

- Good, because identity is total once a workspace is loaded. The contract, the layout
  sidecar and the diagram all key off an ID that is simply there.
- Good, because the reconciliation index is complete the moment it is built, so the
  duplicate detection ADR-0016 relies on is real rather than partial.
- Good, because "unidentified" stops being a state the UI has to explain. It is a
  loading state, and the honest thing to render during it is that the workspace is
  still opening.
- Bad, because **opening a model the tool has not seen before modifies every file in
  it.** The first commit after adopting `sv2` on an existing repository is large,
  mechanical, and touches everything. That has to be told to the user before it
  happens, not discovered in `git status` afterwards.
- Bad, because a workspace that cannot be written — read-only checkout, permissions,
  a mounted artifact — cannot be opened the ordinary way. It must degrade to a
  read-only session whose identities live in memory and are never written, and in
  which anything that would persist a handle is unavailable rather than silently
  ineffective.
- Bad, because it spends the keyspace on elements nobody has pointed at yet. ADR-0016
  sizes it at 1.26 billion, so this is a note rather than a risk.
- Neutral for [ADR-0001](0001-text-is-authoritative.md). The allocation is an ordinary
  text edit on the ordinary path, the tree stays lossless
  ([ADR-0004](0004-lossless-syntax-tree.md)), and the file remains the model. It is
  unrequested, which is a product problem, not an architectural one.

### Confirmation

| ID    | Fitness function | Method |
| ----- | ---------------- | ------ |
| FIT-1 | Allocation is idempotent | Open a workspace twice; the first open produces a diff, the second produces none |
| FIT-2 | Allocation does not reformat | `parse(s).text() == s` holds for every file after allocation, and the diff contains only inserted notes |
| FIT-3 | Identity is total after load | No element reachable from a loaded workspace carries the `unidentified` arm |
| FIT-4 | No collisions | No ID appears twice across a workspace after allocation, over a generated workspace larger than any real one |
| FIT-5 | Read-only degrades rather than fails | A workspace whose files cannot be written opens, reports itself read-only, and leaves every file byte-identical |

## Pros and Cons of the Options

### Option 1 — Eager

- Good, because identity is total, so DD-1 and DD-3 need no discipline.
- Good, because the transient state is a loading state, which the UI has anyway.
- Bad, because opening is a write, and that surprises people.
- Bad, because read-only workspaces need a second, defined mode.

### Option 2 — Lazy

- Good, because opening a model never modifies it, which is what people expect of
  anything that calls itself an editor.
- Good, because it is reversible: eager allocation can be adopted later, whereas files
  already rewritten cannot be un-rewritten.
- Bad, because `unidentified` becomes permanent and every consumer carries it (DD-1,
  DD-2).
- Bad, because the reconciliation index is incomplete, weakening duplicate detection
  (DD-3).

### Option 3 — Per file, on first change

- Good, because churn is proportional to what the user actually touched.
- Neutral, because it has most of Option 2's bookkeeping with less of its predictability.
- Bad, because the trigger is a rule ADR-0016 does not contain, so it would need its
  own justification, and "the file you edited also gained forty unrelated notes" is
  not obviously less surprising than the eager case.

## More Information

Two things this record deliberately does not decide:

- **How the user is told.** DD-4 requires that the write be visible before it happens.
  Whether that is a prompt on first open, a status-bar notice, or a preference is a UI
  decision and belongs with the shell.
- **What read-only mode can do.** FIT-5 fixes that such a workspace opens and writes
  nothing. Which operations are unavailable in it — diagram layout certainly, since it
  has nowhere to key — is not specified here.

### Related decisions

- [ADR-0016](0016-element-identity-via-petname-notes.md) — what an identity is; this
  record answers *when* one is allocated, which that one left open.
- [ADR-0017](0017-view-scoped-json-lines-sidecar-for-diagram-layout-and-styling.md) —
  the sidecar that keys off these IDs, and the reason DD-1 matters in the diagram.
- [ADR-0002](0002-ir-admits-what-parses.md) — the other partial state every consumer
  already carries, which DD-2 is about not multiplying.
