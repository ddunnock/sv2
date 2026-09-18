---
title: "ADR-0017: Use view-scoped JSON Lines sidecar files for diagram layout and styling"
status: "proposed"
date: 2026-09-17
version: "1.0"
decision-makers: David Dunnock
consulted: TBD
informed: TBD
---

# ADR-0017: Use view-scoped JSON Lines sidecar files for diagram layout and styling

## Context and Problem Statement

Diagram placement and styling cannot live in the SysML v2 textual notation. The OMG
notation has no representation for symbol geometry, and any encoding invented for it
would either break external tool round-trip or be discarded on import. A sidecar
alongside the model is therefore required.

[ADR-0016](0016-element-identity-via-petname-notes.md) placed stable element identifiers in
inline notes within the declaring `.sysml` or `.kerml` file. Those identifiers are the
keys this sidecar uses, so the sidecar carries no naming scheme of its own and survives
element renames for free.

What remains to be decided is the serialization format, the file granularity, and the
rules that keep the file reviewable in version control. The last is the binding
constraint: this tool exists in part because a binary model file makes a merge request
unreviewable, and a sidecar that conflicts on every concurrent edit would reproduce that
failure in a new encoding.

## Decision Drivers

* **DD-1 — Independent merge granularity.** Two engineers moving different nodes in the
  same view must merge without conflict and without a custom Git merge driver. This
  makes the smallest independently mergeable unit the primary format criterion.
* **DD-2 — Views are cross-cutting; declarations are not.** A single view draws elements
  declared across many files. Pairing a sidecar to one declaring file is structurally
  wrong, and pairing to the model file also couples the sidecar to that file's path,
  which breaks on rename or move.
* **DD-3 — Serialization must be idempotent.** Layout engines emit floating-point
  coordinates. Persisting them verbatim means every layout run rewrites the file with
  differences in the low decimals, which destroys reviewability regardless of format.
* **DD-4 — The sidecar is regenerable, not authoritative.** Deleting it must degrade the
  diagram to auto-layout, never corrupt or lose the model.
* **DD-5 — External tool neutrality.** The `.sysml` and `.kerml` files must remain
  parseable by the OMG pilot implementation and Cameo with the sidecar absent. This is
  already satisfied by keeping layout out of the model file and must not regress.
* **DD-6 — Single serialization stack.** One engineer sustains this. Every additional
  format is another parser, canonicalizer, and set of edge cases.
* **DD-7 — Offline and toolchain-free.** The file must be readable and diffable with
  nothing but a text editor and Git.

## Considered Options

* **Option 1 — View-scoped JSON Lines**, one record per line, canonically sorted.
* **Option 2 — Pretty-printed JSON**, one document per view.
* **Option 3 — TOML** with arrays of tables.
* **Option 4 — YAML.**
* **Option 5 — Embed layout in inline notes** in the model file, reusing the ADR-0016
  mechanism.

## Decision Outcome

Chosen option: **Option 1, view-scoped JSON Lines.**

It is the only text format considered where a node's entire placement occupies exactly
one line, which makes Git's line-based three-way merge resolve independent node moves
automatically (DD-1). It is serde-native, so it costs no additional dependency or parser
(DD-6), and it remains readable in a diff without tooling (DD-7).

### File layout and naming

Sidecars are keyed by view identifier, not by model file path (DD-2):

```
views/<view-id>.layout.jsonl     placement: node geometry, edge waypoints, collapse state
views/<view-id>.style.jsonl      styling: colors, line styles, compartment visibility
```

`<view-id>` is the view element's identifier under ADR-0016. Placement and styling are
separate files because they have different volatility and different value: placement is
derived and safe to regenerate, styling is authored intent and expensive to lose.
Deleting the layout file must be a safe operation; deleting the style file must not be
required in order to do it.

### Record shape

The first line is a header record. Remaining lines are element records, sorted by
`(kind, element)`.

```jsonl
{"kind":"header","schema":1,"view":"amber-lattice-003","engine":"orthogonal","engine_version":"0.4.0","grid":8}
{"kind":"node","element":"crisp-harbor-042","x":320,"y":176,"w":160,"h":96,"src":"pinned"}
{"kind":"node","element":"quiet-meadow-117","x":544,"y":176,"w":160,"h":96,"src":"auto"}
{"kind":"edge","element":"brisk-anchor-008","waypoints":[[480,224],[544,224]],"src":"auto"}
{"kind":"state","element":"crisp-harbor-042","collapsed":false,"compartments":["parts","ports"]}
```

### Normative rules

| ID  | Rule                                                                                                                                                  |
|-----|-------------------------------------------------------------------------------------------------------------------------------------------------------|
| R-1 | All coordinates and dimensions are integers in grid units. The grid size is declared in the header. No floating-point values are written.             |
| R-2 | Records are sorted by `(kind, element)`. Keys within a record are emitted in a fixed declared order.                                                  |
| R-3 | LF line endings, one trailing newline, no trailing whitespace. A `.gitattributes` entry sets `*.jsonl text eol=lf`.                                   |
| R-4 | Every element record carries `src`, valued `auto` or `pinned`. A re-layout command discards `auto` records and preserves `pinned` ones.               |
| R-5 | The header stamps the layout engine and its version, so a stored snapshot that predates an engine change is detectable rather than silently reflowed. |
| R-6 | Unknown record kinds and unknown fields are preserved on round-trip, so a newer schema written by another instance is not destroyed by an older one.  |
| R-7 | `schema` is an integer and is incremented on any breaking change to record shape.                                                                     |

R-4 resolves the tension between storing every position and storing only overrides.
A full snapshot is written so that rendering is reproducible without re-running layout,
but provenance is recorded per record so that human decisions are distinguishable from
machine output and can be selectively discarded.

### Consequences

* Good, because concurrent moves of different nodes merge cleanly with stock Git and no
  merge driver (DD-1).
* Good, because view-scoped naming means renaming or moving a `.sysml` file has no
  effect on the sidecar (DD-2).
* Good, because integer grid coordinates make re-layout a no-op in the diff when nothing
  moved (DD-3).
* Good, because deleting the layout file degrades to auto-layout with no model impact
  (DD-4).
* Good, because splitting styling from placement means the volatile file can be
  regenerated without risking authored intent.
* Bad, because JSON Lines supports no comments. Any explanatory text must be carried in
  an explicit field, and the file is machine-oriented rather than hand-authored.
* Bad, because two files per view roughly doubles file count in a large project.
* Bad, because records can be orphaned when elements are deleted from the model, which
  requires a garbage-collection command rather than being self-correcting.
* Neutral, because snapping to a grid constrains placement precision. For rule-based
  orthogonal layout this is a benefit, but it forecloses free-form pixel positioning.

### Confirmation

| ID    | Fitness function                   | Method                                                                                                                                  |
|-------|------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------|
| FIT-1 | Serialization is idempotent        | Parse then re-serialize every sidecar in the test corpus; assert byte identity                                                          |
| FIT-2 | Re-layout is stable                | Run auto-layout twice on an unchanged model; assert the emitted file is byte-identical                                                  |
| FIT-3 | Concurrent edits merge cleanly     | CI test creating two branches that each move a different node, then asserting `git merge` succeeds with no conflict                     |
| FIT-4 | No orphaned records                | Validation pass asserting every `element` in a sidecar resolves to an ID present in the model; reported as a warning with a `gc` remedy |
| FIT-5 | External tool neutrality preserved | Parse the model files with the OMG pilot implementation with sidecars absent; assert clean parse                                        |
| FIT-6 | Regenerability                     | Delete all layout files, open every view, assert each renders without error                                                             |
| FIT-7 | Forward compatibility              | Round-trip a file containing an unknown record kind and unknown fields; assert preservation per R-6                                     |

## Pros and Cons of the Options

### Option 1 — View-scoped JSON Lines

* Good, because one node equals one line, giving the finest practical merge granularity.
* Good, because `serde_json` handles it with no added dependency.
* Good, because insertion of a new element is a single-line insertion at a sorted
  position, which is inherently conflict-free against edits elsewhere.
* Neutral, because it is machine-oriented; readable in a diff, not pleasant to hand-author.
* Bad, because it supports no comments.

### Option 2 — Pretty-printed JSON

* Good, because it is the most familiar format and trivially serde-native.
* Good, because nesting can express view and element hierarchy directly.
* Bad, because object and array boundaries put braces and commas on shared lines, so
  edits to adjacent elements conflict even when the elements are unrelated (DD-1).
* Bad, because key ordering is unstable unless explicitly controlled, producing diff
  noise unrelated to any change (DD-3).

### Option 3 — TOML with arrays of tables

* Good, because it is the most human-readable option and supports comments.
* Good, because `[[node]]` blocks are line-oriented, so merge behavior is respectable.
* Neutral, because each element occupies roughly six lines instead of one, making files
  several times longer.
* Bad, because it adds a second serialization stack if JSON is used anywhere else (DD-6).
* Bad, because deeply nested structures such as edge waypoint lists are awkward to
  express and to read.

### Option 4 — YAML

* Good, because it is readable and supports comments.
* Bad, because indentation-sensitive merges are the worst of the options considered; a
  conflict in one block can corrupt the structure of neighbors.
* Bad, because implicit typing has well-known ambiguity hazards for unquoted scalars.

### Option 5 — Embed layout in inline notes in the model file

* Good, because it reuses the ADR-0016 mechanism exactly, with one hiding and stripping
  implementation rather than two.
* Good, because there is no file pairing and therefore no desynchronization on rename.
* Bad, because views are cross-cutting: a view containing elements from six files has no
  single correct file to store its layout in (DD-2).
* Bad, because layout churn would produce diffs in the semantically meaningful file,
  making model review harder rather than easier.
* Bad, because layout data would travel into external tools that have no use for it,
  increasing the surface where ADR-0016's benign-in-external-tools property must hold.
* Bad, because the model file could not be regenerated or reset independently of layout
  (DD-4).

## More Information

### Risks

| ID          | Risk                                                       | Handling                                                                                                 |
|-------------|------------------------------------------------------------|----------------------------------------------------------------------------------------------------------|
| RISK-0017-1 | Records orphaned by element deletion accumulate            | `gc` command plus FIT-4 warning in CI                                                                    |
| RISK-0017-2 | Layout engine change silently reflows stored positions     | Engine version stamped in header (R-5); mismatch surfaces a prompt rather than a silent rewrite          |
| RISK-0017-3 | Pinned records accumulate and fight the rule set over time | Provenance field makes them enumerable; provide a "release all pins in this view" action                 |
| RISK-0017-4 | Sidecar not committed, so layout is lost between machines  | Regenerable by design (DD-4); document that sidecars belong in version control and are not to be ignored |
| RISK-0017-5 | Windows CRLF normalization rewrites whole files            | `.gitattributes` per R-3; FIT-1 catches regressions                                                      |
| RISK-0017-6 | Two files per view doubles file count at scale             | Accepted; revisit if a project exceeds roughly 500 views                                                 |

### Assumptions

| ID    | Assumption                                                                  | Basis                                                   | Impact if wrong                                                                                            |
|-------|-----------------------------------------------------------------------------|---------------------------------------------------------|------------------------------------------------------------------------------------------------------------|
| A-001 | Views are first-class and carry stable identifiers under ADR-0016           | Follows from ADR-0016 and the view-tab UI model         | Without stable view IDs the file naming scheme needs a different key                                       |
| A-002 | A view may reference elements declared in multiple files                    | SysML v2 view and expose semantics                      | If views were always file-scoped, Option 5 becomes viable                                                  |
| A-003 | Auto layout is deterministic for a fixed engine version                     | Required by FIT-2                                       | Non-determinism would force storing all positions permanently and disable re-layout as a routine operation |
| A-004 | Git is the version control system                                           | Program context                                         | A different VCS with semantic merge could change the DD-1 weighting                                        |
| A-005 | Sidecars are committed, not generated at open time                          | Implied by wanting placement to persist across machines | If treated as local cache, most of DD-1 falls away and the format choice loosens considerably              |

A-005 is the load-bearing assumption. If sidecars are local cache rather than committed
artifacts, this ADR is substantially over-engineered and should be reconsidered.

### Related Decisions

* [ADR-0016](0016-element-identity-via-petname-notes.md) — element identifiers in inline notes.
  This ADR consumes those identifiers as its keys and adds no naming scheme of its own.
* [ADR-0013](0013-rust-cst-via-webassembly-as-code-mirror-syntax-tree-source.md) — the
  Rust core that parses the model and therefore resolves the identifiers referenced here.
  Cited as ADR-0011 until 2026-09-18; ADR-0011 is *Specification BNF as a pinned input*,
  and the filename quoted belonged to no record at all.

### Open Items

* Grid size default is unset. It should be chosen against the connector routing rule set
  rather than picked arbitrarily.
* Whether edge waypoints are stored at all, or always recomputed from node positions and
  the routing rules. Storing them is listed above for completeness, but if routing is
  fully deterministic given node positions, omitting them removes a large share of the
  file's volume and churn.