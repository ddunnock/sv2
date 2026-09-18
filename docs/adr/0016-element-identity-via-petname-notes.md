---
title: "ADR-0016: Stable element identity via petname IDs carried in inline textual notes"
status: "accepted"
supersedes: 0009
date: 2026-09-17
accepted: 2026-09-18
version: 0.1.0
decision-makers: David Dunnock
consulted: ""
informed: ""
---

# ADR-0016: Stable element identity via petname IDs carried in inline textual notes

## Context and Problem Statement

The editor parses `.sysml` and `.kerml` files with its own lexer, parser, AST, and resolver. Several features need a stable handle for every user-authored model element, and that handle must not depend on the element's name:

- **Diagrams.** The layout sidecar stores node positions and styling. A diagram node must survive a rename of its element.
- **Document generation.** Templates such as an ICD must bind to an element's identity (`{{id.name}}`), not to the name in effect when the template was written.

The KerML/SysML v2 textual notation has no syntax for `elementId` ([OMG KerML issue tracker](https://issues.omg.org/issues/spec/KerML)). An identifier generated in memory is therefore lost on reparse unless it is persisted.

**Decision:** How should element identity be represented, persisted in the model files, and allocated, so that it survives renames and branching while staying invisible to, and harmless in, other SysML v2 tools?

## Decision Drivers

- **DD-1 Rename stability.** Identity must survive changes to declared names and short names.
- **DD-2 External-tool benignity.** Files carrying IDs must open in other SysML v2 tools with no new diagnostics, no change to name resolution, and no added model elements. Other tools are not expected to use the ID.
- **DD-3 Clean authoring surface.** IDs must be hideable in this editor and removable on demand for export.
- **DD-4 Git-friendly.** Branching, merging, and copy/paste must not silently produce duplicate identities.
- **DD-5 Human-usable.** IDs must be readable and speakable in templates, reviews, and diagnostics.
- **DD-6 Interoperability.** A Systems Modeling API–compatible `elementId` (UUID) must be derivable when needed.
- **DD-7 Self-contained files.** Identity must survive edits made outside this editor as far as practical.

## Considered Options

1. **OPT-1 Petname ID in an inline textual note, with random allocation and duplicate repair**
2. **OPT-2 Petname ID embedded in the short name** (`<'1.1@maple-sunrise-314'>`)
3. **OPT-3 Metadata annotation in the text** (for example, `@IdentityMetadata::ElementId`)
4. **OPT-4 Sidecar-only mapping** (no identity in the model text)
5. **OPT-5 Qualified-name-derived identity** (no stored identity)

## Decision Outcome

Chosen option: **OPT-1, "Petname ID in an inline textual note, with random allocation and duplicate repair."**

- A note is discarded lexically by conforming tools, which satisfies DD-2 without the name-resolution side effects of OPT-2 or the model clutter of OPT-3.
- Because the note travels with the declaration in the file, the ID survives renames (DD-1), external edits that preserve notes (DD-7), and git operations (DD-4).
- The editor can conceal the notes or strip them (DD-3).
- Random allocation combined with duplicate detection and repair handles branch merges and copy/paste, which a counter cannot (DD-4).

### Specification

#### ID format

| Item | Rule |
|---|---|
| Grammar | `^[a-z]+-[a-z]+-[0-9]{3}$`, in the form `<adjective>-<noun>-<ddd>` |
| Example | `maple-sunrise-314` |
| Vocabulary | Seeded from the `petname` crate (v3.2.0, Apache-2.0) medium lists: 1,198 adjectives and 1,052 nouns, all lowercase ASCII, no duplicates, no adjective/noun overlap |
| Keyspace | 1,198 × 1,052 × 1,000 = 1,260,296,000 |
| Validation | Grammar only. IDs are stored as strings and never decoded to indices, so the vocabulary can change without invalidating existing IDs. |
| Vocabulary control | Vendored, versioned, and curated (offensive, homophone, and confusable pairs removed). It affects new allocations only. |

#### Carrier and placement

The ID is written inline in the file that declares the element, whether that file is `.sysml` or `.kerml`. No companion or auxiliary model file is created. Notes are a lexical construct defined by KerML and reused by the SysML v2 textual notation, so the same carrier and rules apply to both file types.

| Item | Rule |
|---|---|
| Location | The declaring file itself (`.sysml` or `.kerml`) |
| Canonical form | Block note, `//* @id <ID> */`, placed immediately before the declaration it identifies, on the same line |
| Accepted on read | Line note, `// @id <ID>`, on the line immediately preceding the declaration. The formatter normalizes it to the canonical form. |
| Forbidden | Regular comments (`/* ... */`), which become `Comment` elements in the model, and any other carrier |
| Marker | The note body must match `^\s*@id\s+<ID>\s*$`. Other notes are ignored as ordinary notes. |
| Attachment | The note attaches to the member declaration whose first non-trivia token follows it. That token may be a visibility keyword, a prefix keyword, or a metadata prefix. |
| Inline relationships | For relationships without a standalone declaration (for example `:>`, `:>>`, `:`), the note immediately precedes the relationship operator. |
| Lexer | Notes are retained as trivia on the adjacent token so that attachment is deterministic. |

```sysml
package Vehicles {
    //* @id maple-sunrise-314 */ part def Vehicle {
        //* @id quiet-harbor-027 */ part engine //* @id amber-falcon-902 */ : Engine;
    }
}
```

#### Identity scope

| Element category | Identity source |
|---|---|
| User elements and explicit relationships with textual declarations | Stored petname ID (this ADR) |
| Owning memberships of identified elements | Derived: `<owned-ID>/m`. Not stored. |
| Implied and derived relationships | Derived: stable hash of `(source-ID, relationship kind, target-ID, ordinal)`, in a separate namespace from petname IDs. Not stored. |
| Standard library elements | Normative name-based UUIDs defined by KerML. Never assigned a petname ID. |

#### Allocation

1. Generate a candidate ID randomly from the vocabulary.
2. Reject the candidate and retry if the resolver's workspace-wide ID index already contains it.
3. Write the ID note when the element is created in the editor, or when an unidentified element is first resolved and reconciliation (below) finds no match.

Within a single workspace, the index check prevents collisions. Collisions can still arise from allocations made independently on separate branches or clones, which are not checked against each other. For *n* independently allocated IDs, the probability of any collision is about *n*² / (2 × 1.26 × 10⁹):

| Independently allocated IDs (*n*) | Approximate collision probability |
|---|---|
| 1,000 | 0.04% |
| 10,000 | 4.0% |
| 50,000 | 63% |

These cases are handled by repair rather than prevention.

#### Duplicate repair

- **Diagnostic.** The resolver reports `ID-DUP` for every ID that appears on more than one declaration.
- **Keeper selection.** The occurrence whose qualified name, kind, and structural fingerprint match the sidecar record for that ID keeps the ID. If none or several match, the first occurrence in (file path, byte offset) order keeps it.
- **Reassignment.** All other occurrences receive fresh IDs through a quick fix. The editor applies this automatically for duplicates introduced by a paste within the editor.

#### Loss recovery and reconciliation

The sidecar records, for each ID, the element's last-known qualified name, kind, file, and structural fingerprint. The fingerprint is built from the kind, the owner's ID, the typing and specialization targets, and the owned member names.

When an element has no ID, which happens after an external tool drops notes or after an ID strip, the editor reconciles it in order:

1. **Exact match.** If the qualified name and kind match an unclaimed sidecar record, the element reuses that ID.
2. **Fingerprint match.** If exactly one unclaimed record's fingerprint matches above the threshold, the element reuses that ID.
3. **No match.** Otherwise, the element receives a new ID and an informational diagnostic is emitted.

#### Editor behavior and export

| Capability | Behavior |
|---|---|
| Concealment | `@id` notes are hidden by default, with a toggle to show them |
| Strip | Removes every `@id` note. The sidecar retains the map. |
| Re-import | Restores IDs through reconciliation |
| API export | `elementId = UUIDv5(project-namespace-UUID, ID)`. The project namespace UUID is stored in the project configuration. |
| Templates | Bind by ID, for example `{{maple-sunrise-314.name}}`, resolved at generation time |
| Layout sidecar | Keyed by ID |

#### Diagnostics

| Code | Condition |
|---|---|
| `ID-MALFORMED` | An `@id` note whose ID does not match the grammar |
| `ID-DUP` | The same ID on more than one declaration |
| `ID-ORPHAN` | An `@id` note that does not attach to a declaration |
| `ID-ON-LIBRARY` | An `@id` note on a standard library element |

### Consequences

- Good, because renames no longer break diagram layout or template bindings.
- Good, because other tools see no ID-related text, which keeps the files portable.
- Good, because IDs live in the files, so they survive git operations and editing outside this editor that preserves notes.
- Good, because IDs are readable in templates, diagnostics, and reviews.
- Good, because the vocabulary can evolve, since stored IDs are never decoded.
- Bad, because the model text gains visible content when viewed outside this editor.
- Bad, because formatters or refactorings in other tools may move or drop notes, which makes reconciliation necessary.
- Bad, because independent allocations can collide (about 4% at 10,000 IDs), which requires the repair workflow.
- Bad, because reconciliation is heuristic and can occasionally assign a new ID where the user expected the old one.
- Neutral, because IDs are handles, not secrets. The keyspace is enumerable, and IDs must never be used for authorization.

### Confirmation

- **Round-trip.** Property tests (`proptest`) confirm that parse → format → parse preserves every ID and its attachment.
- **Strip and re-import.** On unmodified files, stripping and re-importing restores 100% of IDs.
- **External benignity.** Fixture files with IDs open in at least two other SysML v2 tools (for example, the Pilot Implementation and Syside) with no new diagnostics, no added `Comment` elements, and unchanged name resolution.
- **Merge.** Two branches that allocate the same ID produce `ID-DUP`, and the quick fix leaves exactly one keeper that matches the sidecar.
- **Rename.** Renaming an element leaves its diagram node, position, and template bindings intact.
- **Library.** No standard library element receives or accepts a petname ID.

## Pros and Cons of the Options

### OPT-1: Petname ID in an inline textual note, with random allocation and duplicate repair

The chosen option, as specified above.

- Good, because notes are discarded lexically, so there is no semantic footprint in other tools (DD-2).
- Good, because the note can be placed inline, which covers relationships that have no identification slot.
- Good, because it has no quoting or escaping interactions with names.
- Neutral, because it depends on a trivia-preserving lexer, which the editor already has.
- Bad, because other tools may drop or move notes.

### OPT-2: Petname ID embedded in the short name

Append the ID inside the short name, for example `<'1.1@maple-sunrise-314'>`, and hide it in the editor.

- Good, because the ID sits in a slot the declaration already has.
- Bad, because the short name takes part in name resolution. References to `'1.1'` fail to resolve in other tools once the declaration becomes `'1.1@…'` (violates DD-2).
- Bad, because the suffix masks distinguishability violations. Two elements both named `<'1.1'>` appear distinct to other tools.
- Bad, because most elements have no short name, and inline relationships have no slot for one, so the IDs would have to be injected everywhere anyway.
- Bad, because it needs basic-name-to-quoted-name conversion and escaping rules for names that already contain `@`.

### OPT-3: Metadata annotation in the text

Declare identity with a metadata usage, as OpenSysML does with `@IdentityMetadata::ElementId` ([Open-MBEE/OpenSysML PR #245](https://github.com/Open-MBEE/OpenSysML/pull/245)).

- Good, because it is semantically explicit and interoperable with tools that recognize the annotation.
- Bad, because it adds model elements that other tools parse, resolve, and display (violates DD-2 and DD-3).
- Bad, because it requires the annotation's library definition to be available to other tools.
- Bad, because it adds substantial clutter to every declaration.

### OPT-4: Sidecar-only mapping

Keep the model text free of IDs and map identity to elements in the sidecar only.

- Good, because the model text stays completely unchanged.
- Bad, because renames made outside this editor are undetectable, and every such rename relies on heuristic reconciliation (violates DD-1 and DD-7).
- Bad, because the files alone do not carry identity, so moving a file without its sidecar loses identity.

### OPT-5: Qualified-name-derived identity

Compute identity from the qualified name, with no stored state.

- Good, because it requires no storage and no allocation.
- Bad, because a rename changes the identity, which defeats the purpose (violates DD-1).

### Sub-decision: allocation strategy

| Strategy | Assessment |
|---|---|
| **Random with index check and duplicate repair (chosen)** | Needs no coordination between branches. Collisions across branches are detected and repaired. |
| Counter with bijective permutation | Collision-free on a single line of history, but two branches allocate the same counter values, so every merge produces duplicates. It also ties stored IDs to vocabulary order. Rejected under DD-4. |
| Qualified-name hash | Identical to OPT-5. Rejected. |

## More Information

### Confirmed before acceptance

- **OI-1 — CONFIRMED, 2026-09-18.** Notes are discarded lexically in both grammars, and
  a regular comment is not. Checked mechanically against the frozen derived grammar
  (`.claude/state/grammar/units`, sha256 `0b9abe64f1f3ce2b`) rather than by reading
  prose, because "no production can consume this terminal" is a property of the whole
  grammar and not of any one clause:

  | Terminal | Referenced by, of 554 live units | Consequence |
  |---|---|---|
  | `SINGLE_LINE_NOTE` | none | discarded lexically, both grammars |
  | `MULTILINE_NOTE` | none | discarded lexically, both grammars |
  | `REGULAR_COMMENT` | `Comment`, `Documentation`, `TextualRepresentation`, all shared | model content, both grammars |

  This confirms both halves of the claim the carrier rests on: the chosen carrier adds
  nothing a conforming tool can see, and the forbidden one would have added a `Comment`
  element to every identified declaration. It also confirms the rule reaches `.kerml`
  files as much as `.sysml`, since the three consuming units are shared between the
  grammars rather than scoped to one.

### Open items

These are implementation tasks, not conditions on the decision. None of them can change
the carrier, the format, or the allocation strategy.

- **OI-2.** Verify the short-name name-resolution and distinguishability behavior cited against OPT-2. It concerns a rejected option, so it can only strengthen the argument against OPT-2, never reopen OPT-1.
- **OI-3.** Set the fingerprint-match threshold and the fingerprint contents from test data.
- **OI-4.** Include Apache-2.0 attribution (NOTICE) for the vendored `petname` word lists.

### Review triggers

- A future KerML or SysML v2 revision adds textual syntax for `elementId` (see the KerML issue tracker). This ADR should be revisited in that case.
- Other tools are observed stripping or relocating notes often enough to degrade reconciliation.

### References

- `petname` crate: [crates.io/crates/petname](https://crates.io/crates/petname), source at [github.com/allenap/rust-petname](https://github.com/allenap/rust-petname)
- OMG KerML issue tracker, textual notation lacks `elementId`: [issues.omg.org/issues/spec/KerML](https://issues.omg.org/issues/spec/KerML)
- Open-MBEE OpenSysML, declared and normative element IDs: [PR #245](https://github.com/Open-MBEE/OpenSysML/pull/245)

### Related Decisions

- [ADR-0009](0009-element-identity.md) — the question this record answers, and which it
  supersedes. It reached the same layered shape and assumed a different carrier.
- [ADR-0017](0017-view-scoped-json-lines-sidecar-for-diagram-layout-and-styling.md) — the
  layout sidecar, keyed by the ID defined here, and the store for the last-known name and
  fingerprint that reconciliation reads.
- [ADR-0004](0004-lossless-syntax-tree.md) — the lossless tree is what makes this carrier
  possible at all. A parser that discarded trivia could not round-trip an `@id` note, which
  is what OPT-1's "depends on a trivia-preserving lexer" means.

### Requirements Traceability

- None assigned yet. Link requirement IDs when the identity requirements are baselined.