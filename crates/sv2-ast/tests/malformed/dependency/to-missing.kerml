// Clause: KerML 8.2.3.2 --
//
//   Dependency = ( ownedRelationship += PrefixMetadataAnnotation )*
//                'dependency' ( Identification? 'from' )?
//                client += [QualifiedName] ( ',' client += [QualifiedName] )* 'to'
//                supplier += [QualifiedName] ( ',' supplier += [QualifiedName] )*
//                RelationshipBody
//
// Two lists, the clients and the suppliers, with `to` between them. KerML writes them
// inline, so they are the Dependency's own children. This file leaves out the `to`:
// `dependency a b;`, then `dependency c to d;`. No corpus file leaves a dependency's `to`
// out (every dependency in the pinned corpus writes one), so the corpus never shows
// which list a name after a missing `to` joins.
//
// Recovery (NOT a clause, and NOT an ADR): `dependency_declaration_parts`
// (crates/sv2-syntax/src/parser/definition.rs) reads a list, expects `to`, and reads a
// list. `a` ends the first list, since no `,` follows it; `expect_keyword("to")` reports
// at `b` and consumes nothing, and the second list begins at `b`. So `b` is a supplier,
// not a client, and no `to` is in the tree:
//
//   NonFeatureMember
//     Dependency "dependency a b;"
//       KwDependency "dependency"
//       QualifiedName "a"            <- clients(): the first list, `a` alone
//       QualifiedName "b"            <- suppliers(): the second list, begun at `b`
//       RelationshipBody ";"         <- body(): the `;`
//   NonFeatureMember
//     Dependency "dependency c to d;"  <- whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding two Dependencies.
//  2. The first has no `to`: to_keyword() is None, and something reported it, one
//     diagnostic, on `b`. Its names are split by the lists the parser read, `a` a client
//     and `b` a supplier, not by where a `to` would have been.
//  3. The neighbours still answer. The first's body is its `;`, and the second is whole:
//     `c` to `d`, with its `to`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.3.2 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	dependency a b;
	dependency c to d;
}
