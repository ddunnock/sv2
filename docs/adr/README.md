# Architecture decision records

MADR 4.0.0. One record per decision, numbered, never renumbered.

`status` is one of `proposed`, `accepted`, `superseded`, `rejected`. A superseded record
names its successor in `superseded-by`, and the successor names it back in `supersedes`;
`.claude/scripts/index_decisions.py` fails the gate when only one side of that says so,
and when a `[ADR-NNNN](file.md)` link points at a missing file or at a different record
than its text names. Only `proposed` counts as open.

These are the decisions the front end is built on. If a change would contradict one,
stop and say so — do not implement around it. Superseding a record is fine; doing it
silently is not.

| ID                                          | Decision                                        | Status       |
| ------------------------------------------- | ----------------------------------------------- | ------------ |
| [0001](0001-text-is-authoritative.md)       | Text is authoritative; diagrams are projections | accepted     |
| [0002](0002-ir-admits-what-parses.md)       | The IR admits everything that parses            | accepted     |
| [0003](0003-ir-is-sysml-abstract-syntax.md) | The IR is SysML v2 abstract syntax              | accepted     |
| [0004](0004-lossless-syntax-tree.md)        | Lossless incremental syntax tree, not an AST    | accepted     |
| [0005](0005-sidecar-split.md)               | Layout and styling in a split sidecar           | accepted     |
| [0006](0006-view-membership-in-model.md)    | Diagram membership stays in the model           | accepted     |
| [0007](0007-local-resolver.md)              | The resolver is local and owned                 | accepted     |
| [0008](0008-first-view-structure.md)        | First view is structure, interconnection second | accepted     |
| [0009](0009-element-identity.md)            | Element identity strategy                       | superseded by [0016](0016-element-identity-via-petname-notes.md) |
| [0010](0010-grammar-and-metamodel-sourcing.md) | Grammar and metamodel sourcing | accepted |
| [0011](0011-specification-bnf-as-a-pinned-input.md) | Specification BNF as a pinned input | accepted |
| [0013](0013-rust-cst-via-webassembly-as-code-mirror-syntax-tree-source.md) | Rust CST via WebAssembly as CodeMirror's syntax tree | **proposed** |
| [0014](0014-kerml-and-sysml-are-two-grammars.md) | KerML and SysML are two grammars sharing a vocabulary | accepted |
| [0015](0015-a-production-belongs-to-the-grammars-that-reach-it.md) | A production belongs to the grammars that reach it | accepted |
| [0016](0016-element-identity-via-petname-notes.md) | Stable element identity via petname IDs in inline notes | accepted |
| [0017](0017-view-scoped-json-lines-sidecar-for-diagram-layout-and-styling.md) | View-scoped JSON Lines sidecar for layout and styling | **proposed** |
| [0018](0018-a-binary-per-process-shape.md) | A binary per process shape, and every binary a shim | accepted |
| [0019](0019-extensions-are-in-tree-consumers-of-the-resolved-model.md) | Extensions are in-tree consumers of the resolved model | **proposed** |
| [0020](0020-element-identity-is-allocated-when-a-workspace-opens.md) | Element identity is allocated when a workspace opens | accepted |
| [0021](0021-tauri-2-hosts-the-studio-window.md) | Tauri 2 hosts the studio window | accepted |
