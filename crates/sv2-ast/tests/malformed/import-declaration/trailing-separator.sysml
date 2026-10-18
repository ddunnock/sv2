// Clause: KerML 8.2.3.4.2 and SysML 8.2.2.5.1 --
//
//   ImportDeclaration = MembershipImport | NamespaceImport
//   MembershipImport  = importedMembership = [QualifiedName] ( '::' isRecursive ?= '**' )?
//   NamespaceImport   = importedNamespace  = [QualifiedName] '::' '*'
//                                           ( '::' isRecursive ?= '**' )?
//
// An ImportDeclaration's own shape cannot be wrong: it is one alternative and nothing
// else, and every one of the 756 in this repository holds exactly ONE child node and NO
// token of its own -- the separators and stars belong to the alternative inside it.
// tests/sweep.rs asserts both over every tree.
//
// The empty-but-present case is already pinned, for the Import kind, by
// tests/malformed/import/declaration-names-nothing.sysml (`public import ;`), so this
// fixture does not repeat it. It pins the other thing a caller can get wrong here, and
// the reason it is worth a file of its own is that NOTHING ABOUT THE TREE LOOKS DAMAGED.
//
// Recovery (NOT a clause, and NOT an ADR): `qualified_name`
// (crates/sv2-syntax/src/parser/import.rs) only takes a `::` into the name when a NAME
// follows it, so here the name ends at `A` and the separator is left. `import_suffix`
// then looks for what follows and finds no `*`, so `imported` closes the checkpoint as a
// MembershipImport -- the alternative that parsed, which is not the one the trailing `::`
// suggests a reader was reaching for:
//
//   Import "public import A::;"
//     VisibilityIndicator "public"
//     ImportDeclaration "A::"          <- non-empty, and no tokens of its own
//       MembershipImport "A::"         <- the `::` lives HERE
//         QualifiedName "A"            <- one segment, and the `::` is not in it
//
// Three things make that worth pinning:
//
//  1. `imported()` returns Some, the child is a complete MembershipImport, and its
//     QualifiedName names `A`. Every accessor answers plausibly. A caller switching on
//     the child's kind to decide "membership or namespace" gets MembershipImport, and
//     nothing in the tree says that the author wrote a separator and then stopped.
//     Parse::errors() is the only signal, which is why the test reads it.
//  2. The `::` is a token of the ALTERNATIVE, not of the declaration. An accessor on
//     ImportDeclaration that went looking for a separator or a star to decide the
//     alternative itself would be reaching across a production boundary, and would find
//     the tokens in the child rather than in itself.
//  3. The declaration's text ("A::") is WIDER than the name it contains ("A"). Text width
//     is therefore not evidence about what was named, in either direction: the
//     declaration-names-nothing case is zero width with a child, and this one is
//     non-empty with a name shorter than itself.
//
// Do not cite 8.2.3.4.2 for the recovery outcome, and do not cite ADR-0022, which is
// about deviation diagnostics and says nothing about recovery. ADR-0004 supports only the
// shape: every byte reaches the tree, so `public import A::;` round-trips.
public import A::;
