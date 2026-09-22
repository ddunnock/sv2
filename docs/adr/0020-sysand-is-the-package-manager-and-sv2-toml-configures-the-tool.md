---
title: "ADR-0020: sysand is the package manager; sv2.toml configures only the tool"
status: "proposed"
date: 2026-09-22
version: "0.1.0"
decision-makers: David Dunnock
consulted: TBD
informed: TBD
---

# ADR-0020: sysand is the package manager; sv2.toml configures only the tool

## Context and Problem Statement

The resolver has to know which files make up a model before it can resolve a name in
one. For a single file that is trivial. For a real project it is not: the model spans the
project's own sources, the standard library, and libraries other people publish, each at
some version. Something has to declare that set, fetch what is not local, and pin it.

SysML v2 does not make this a path problem. An `import` names a namespace, never a file,
and the resolver searches every root namespace of every file it has loaded. So the
question a manifest answers is not "where does this import point" but "which files and
libraries are in the set the resolver searches". Two libraries that both declare a
top-level `package Sensors` cannot be told apart by any qualified name; the conflict has
to be caught when the set is assembled.

The format for a distributable unit already exists. KerML clause 10.3 defines the project
interchange file (`.kpar`): a ZIP holding `.project.json` (name, version, license,
maintainers, and `usage[]`, each a resource IRI with a version constraint) and
`.meta.json` (an index from top-level names to files, plus checksums). The schema is
pinned here as `vendor/omg/20250201/KerML-Model-Interchange.json` (ptc/25-04-20), and
the ten standard libraries are pinned as KPARs.

A package manager built on that format also already exists. Sysand, maintained by
Sensmetry under MIT OR Apache-2.0, was reviewed at commit `0325533` (2026-09-22,
version 0.2.1). It provides:

- a PubGrub version solver and `sysand-lock.toml`, which records each project's pinned
  version, its sources with `kpar_digest`, and the top-level names it `exports`
- a check, when the lockfile is written, that no top-level name is exported by two
  projects (`core/src/commands/lock.rs`)
- the 20250201 standard libraries embedded as known projects, the same revision this
  repository pins (`core/src/stdlib.rs`)
- a package index that must be servable by a plain static-file HTTP server, identity by
  `pkg:sysand/<publisher>/<name>` IRIs, semver version constraints, typed path and KPAR
  usages, and `.workspace.json` workspaces
- a local environment, `.sysand/env.toml` plus `.sysand/lib/`, that `sysand sync`
  populates and that editors read to find dependency sources

It also has limits that matter here:

- `sysand-core` depends on `reqwest-middleware` unconditionally (`core/Cargo.toml`), so
  linking it brings an HTTP client into the dependency graph even with its `networking`
  feature off.
- `sysand-core` is `publish = false`, so it is not on crates.io.
- Its index protocol calls itself "v0, unversioned; breaking changes expected", and
  breaking commits land weekly.
- An index location must be `http` or `https` (`core/src/index_location.rs`). Local
  directory environments and `file://` usages exist, but an index cannot be a directory.
- Archives are verified by digest; nothing is signed.
- The `.meta.json` name index is filled by a lexer-level scan (`core/src/symbols/`), not a
  parser.

Separately, sv2 needs somewhere to keep its own settings: lint levels, formatting, the
house style for diagrams, and the classification policy of ADR-0021. None of those are
package metadata.

## Decision Drivers

- **DD-1: No redundant ecosystem.** SysML v2 tooling is a small community. A second
  package format or index would split it, and rebuilding a solver, lockfile, index
  protocol, and credential store is a large amount of work that buys sv2 nothing.
- **DD-2: No network at runtime (Invariant 6).** No sv2 build, check, resolve, or gate may
  fetch. `scripts/vendor_sync.py` is the only fetching code in this repository.
- **DD-3: The resolver is owned (ADR-0007).** Package management decides which files are
  loaded. It must not decide what they mean.
- **DD-4: The IR admits everything that parses (ADR-0002).** A missing, stale, or
  malformed manifest is a diagnostic, never a reason to stop reading the model.
- **DD-5: Single-maintainer sustainment.** Every surface kept in step with an external
  project that is still at 0.x is ongoing cost.
- **DD-6: Air-gapped and CUI operation.** Whatever sv2 relies on must work inside an
  enclave with no internet.

## Considered Options

1. **An sv2-native package manager**, uv-style: `Model.toml` manifest, sv2 solver,
   lockfile, registry, and `sv2 sync`.
2. **Link `sysand-core` into sv2** and expose `sv2 sync`, `sv2 add`, and so on.
3. **Use sysand as an external tool; sv2 reads the files it writes.** sv2 has no package
   commands. A separate `sv2.toml` holds tool settings only.
4. **Fork sysand** and add sv2's parser to it, with the aim of upstreaming.

## Decision Outcome

Chosen option: **Option 3, sysand as an external tool, with sv2 reading its files.**

Sysand already implements KerML clause 10.3 packaging with the collision check that name
resolution in SysML v2 needs. Reading its on-disk formats gives sv2 the whole ecosystem
with no network code and no code dependency (DD-1, DD-2). This is the relation
rust-analyzer has to Cargo: it reads what the package manager produced and never acts as
one.

Option 1 was rejected under DD-1 and DD-5. Option 2 was rejected under DD-2 and DD-5:
it brings an HTTP client into the graph, needs a git dependency, and couples sv2's
release schedule to a v0 protocol. Option 4 was rejected under DD-5: a fork of a project
with weekly breaking commits stays current only with constant work. Sysand also needs
only top-level names from a parser, which its scan provides cheaply and which also has
to compile to WASM for its JS binding. A parser at 40% production coverage that rejects
valid files would break `sysand include` for its users.

### Definitions

| Term | Meaning |
|---|---|
| Sysand files | `.project.json`, `.meta.json`, `.workspace.json`, `sysand-lock.toml`, and `.sysand/env.toml` |
| Project root | The directory holding `.project.json` |
| Workspace root | The directory holding `.workspace.json` |
| Input set | The files the resolver loads for one project: its own sources, the standard library, and the sources of its direct and transitive usages |
| Implicit project | The input set sv2 builds when no sysand files exist: the files named on the command line or found under the working directory, plus the standard library |

### Normative rules

| ID | Rule |
|---|---|
| R-1 | sv2 performs no network I/O. It has no `sync`, `add`, `remove`, `lock`, or `publish` command. Fetching, solving, locking, and publishing are done with `sysand`. |
| R-2 | sv2 reads sysand files and never writes them. |
| R-3 | A new crate, `sv2-project`, discovers the sysand files and produces the input set as plain data: file paths, project identifiers, versions, exported top-level names, and which usages are direct. It depends on no other sv2 crate, and no core crate depends on it. Adapters pass its output to the resolver. The resolver never reads a configuration or package file. |
| R-4 | The standard library is always sv2's vendored copy under `vendor/omg/`. When the sysand files name a standard-library revision other than the pinned one, sv2 reports a diagnostic and still resolves against the pinned copy. |
| R-5 | A file's names resolve against its own project's sources, the standard library, and the top-level names exported by its project's direct usages. A name reachable only through a transitive usage resolves, and carries a diagnostic naming the usage to add. |
| R-6 | When two projects in the input set export the same top-level name, sv2 reports a diagnostic and loads both. Sysand refuses such a lock, but an environment can be edited by hand, and ADR-0002 forbids refusing to read. |
| R-7 | Missing sysand files mean an implicit project, with no diagnostic. Present but malformed or stale sysand files produce a diagnostic, and sv2 falls back to the implicit project. |
| R-8 | `sv2.toml` holds tool settings only: `[lint]`, `[format]`, `[diagram]`, and `[classification]`. It never holds project identity, version, dependencies, or index locations, which belong to `.project.json` and sysand's own `sysand.toml`. A `[project]`, `[dependencies]`, or `[registries]` table is an error whose message points to sysand. |
| R-9 | `sv2.toml` may sit at the workspace root and at a project root. The project file overrides the workspace file key by key. Unknown keys are reported, not ignored. |
| R-10 | `[lint]` sets levels (`allow`, `warn`, `deny`) for convention checks only. Specification constraints are not lints and cannot be disabled (ADR-0002). Diagnostics that ADR-0021 marks as fixed also cannot be disabled. |
| R-11 | `[format]` applies only when formatting is explicitly requested. No other edit, graphical or textual, reformats text it did not change (Invariant 1). |
| R-12 | `[diagram]` is the project-wide default layer of the style cascade: built-in defaults, then `sv2.toml`, then the view's `views/<view-id>.style.jsonl` (ADR-0017), then per-element entries. Security markings are not styles and are not part of the cascade (ADR-0021). |
| R-13 | Every path in `sv2.toml` is relative to the file that declares it and stays inside the repository. URLs are rejected (Invariant 6). |

R-1 and R-3 together keep the network out: nothing that fetches is compiled into sv2,
and the only thing crossing from sysand to sv2 is files on disk.

R-5 is a tool policy, not a specification rule. The specification says nothing about
which loaded root namespaces a file may see. The strict rule is chosen because a model
that resolves only through a transitive usage breaks when that usage changes its own
dependencies.

### Illustrative sv2.toml

This example is not normative. ADR-0021 defines `[classification]`.

```toml
[lint]
naming-convention = "warn"
unused-import     = "warn"
missing-doc       = "allow"

[format]
indent    = 4
max-width = 100

[diagram]
theme     = "dark"
connector = "orthogonal"
logo      = "assets/logo.svg"
```

### Consequences

- Good, because sv2 works with every project sysand manages, including projects whose
  authors never use sv2 (DD-1).
- Good, because no network code enters sv2's dependency graph (DD-2).
- Good, because the resolver keeps a data-only input and stays independent of any
  package format (DD-3).
- Good, because `sv2.toml` has one concern, and nothing in it can drift from
  `.project.json`.
- Bad, because sv2 depends on file formats that sysand has not frozen. `env.toml` and
  `sysand-lock.toml` are sysand's own. `.project.json` and `.meta.json` are OMG's and
  are stable.
- Bad, because users need two tools installed.
- Bad, because sv2 cannot improve the `.meta.json` name index for users; it can only
  report where the index and the parse disagree.
- Neutral, because `sv2-project` adds a TOML parser to the dependency graph. It is needed
  for `sv2.toml` in any case.

### Confirmation

| ID | Fitness function | Method |
|---|---|---|
| FIT-1 | No network crate in the graph (R-1) | `deny.toml` bans HTTP and socket client crates across the workspace. Adding the ban requires the matching STD-002-RS enforcement change, so `check_standards_config.py` stays green. |
| FIT-2 | Reads real sysand output (R-2, R-3) | A test fixture is a `.sysand/` environment produced by a real `sysand sync`, with the sysand version recorded next to it. Expected input sets come from sysand's documented formats, not from sv2's output. |
| FIT-3 | Format drift is a diagnostic (R-7) | Negative fixtures: a lockfile with an unknown `lock_version`, an `env.toml` naming a missing path, and a malformed `.project.json`. Each yields a diagnostic and an implicit project, never a panic. |
| FIT-4 | Visibility and collisions (R-5, R-6) | Positive and negative fixtures for a direct usage, a transitive-only name, and two projects exporting the same name. |
| FIT-5 | `sv2.toml` stays in scope (R-8, R-9, R-13) | Negative cases: a `[dependencies]` table, an unknown key, an absolute path, and a URL. |
| FIT-6 | Formatting never leaks (R-11) | A graphical edit on a file that violates `[format]` produces a text delta that touches only the edited element. |

## Pros and Cons of the Options

### Option 1: sv2-native package manager

- Good, because the whole experience is under one tool and one manifest.
- Bad, because it duplicates a working, maintained, open-source implementation of the
  same OMG format (DD-1).
- Bad, because a solver, index protocol, lockfile, and credential store are each
  permanent maintenance (DD-5).
- Bad, because it puts fetching code in sv2 (DD-2).

### Option 2: Link sysand-core

- Good, because users get one binary.
- Bad, because `reqwest-middleware` is an unconditional dependency (DD-2).
- Bad, because `sysand-core` is only available as a git dependency and changes weekly
  (DD-5).

### Option 3: Sysand as an external tool; sv2 reads its files

- Good, because it interoperates fully at the cost of reading five file formats.
- Good, because the boundary is files on disk, which is testable with fixtures.
- Bad, because two of those formats are not yet stable.

### Option 4: Fork sysand and add sv2's parser

- Good, because the name index could come from a real parser.
- Bad, because the fork goes stale quickly (DD-5).
- Bad, because sysand would take on a parser that rejects valid input, and would need it
  to compile to WASM.
- Bad, because the upstream maintainer ships a commercial SysML v2 language tool and may
  not want a second parser in sysand. This is an inference that has not been confirmed
  with them.

## More Information

### Contributions upstream

This section is not normative. Work that helps sysand without coupling sv2 to it:

- **Name-index comparison.** Run sysand's scan and sv2's parser over the pinned corpus and
  file each disagreement as an issue with its reproducing file.
- **Signed archives.** Digests prove integrity, not who published a package. Provenance
  matters in an ATO context.
- **Directory indexes.** An index served from a local directory would simplify
  air-gapped installations.
- **A TOML project manifest**, if authoring `.project.json` by hand proves painful.

Each starts as an issue, not a pull request.

### Risks

| ID | Risk | Handling |
|---|---|---|
| RISK-0020-1 | Sysand changes `env.toml` or the lockfile incompatibly | Read the smallest subset of fields that works, keep the tested sysand version beside the FIT-2 fixture, and treat unknown versions under R-7 |
| RISK-0020-2 | Sysand is abandoned | The OMG formats (`.project.json`, `.meta.json`, `.kpar`) remain. `sv2-project` can walk `.project.json` usages over a directory of extracted KPARs without `env.toml`. |
| RISK-0020-3 | A program requires sv2 itself to fetch dependencies | That is a review trigger, not a gradual change. It would be a separate binary under ADR-0018, never part of the core crates. |

### Open Items

- **OI-1.** Confirm sysand's rule for locating `.sysand/` from a project or workspace
  root (`core/src/env/discovery.rs`), and adopt the same rule.
- **OI-2.** Decide whether a disagreement between `.meta.json` and the parsed top-level
  names is a lint or a fixed diagnostic.
- **OI-3.** The lint catalogue and the `[format]` keys, once a formatter exists.
- **OI-4.** The `[diagram]` keys. These wait on ADR-0017 and on reconciling ADR-0005 with
  it (ADR-0019 RISK-0019-4).

### Review Triggers

- Sysand's index protocol reaches v1 and `sysand-core` is published with network code
  optional. Linking it into a separate binary for convenience commands becomes
  reasonable.
- Sysand stops being maintained.
- Sensmetry asks for sv2's parser, which reverses the reasoning for Option 4.

### Related Decisions

- [ADR-0001](0001-text-is-authoritative.md): `sv2.toml` never holds model content.
- [ADR-0002](0002-ir-admits-what-parses.md): R-6, R-7, and R-10.
- [ADR-0004](0004-lossless-syntax-tree.md): R-11.
- [ADR-0007](0007-local-resolver.md): the resolver takes a data-only input set.
- [ADR-0017](0017-view-scoped-json-lines-sidecar-for-diagram-layout-and-styling.md): the
  view style file that R-12 layers over.
- [ADR-0018](0018-a-binary-per-process-shape.md): where a fetching binary would go if
  RISK-0020-3 happens.
- [ADR-0021](0021-security-markings-are-model-metadata.md): the `[classification]`
  table.
