# Architecture decision records

MADR 4.0.0. One record per decision, numbered, never renumbered.

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
| [0009](0009-element-identity.md)            | Element identity strategy                       | **proposed** |
