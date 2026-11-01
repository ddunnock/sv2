// Clause: SysML 8.2.2.3 --
//
//   Dependency            = ( ownedRelationship += PrefixMetadataAnnotation )*
//                           'dependency' DependencyDeclaration RelationshipBody
//   DependencyDeclaration = ( Identification 'from' )?
//                           client += [QualifiedName] ( ',' client += [QualifiedName] )*
//                           'to'
//                           supplier += [QualifiedName] ( ',' supplier += [QualifiedName] )*
//
// The same text as `to-missing.kerml` beside it, `dependency a b;` then `dependency c to
// d;`, read as SysML, which names the declaration as a production of its own. No corpus
// file leaves a dependency's `to` out.
//
// Recovery (NOT a clause, and NOT an ADR): `dependency_declaration`
// (crates/sv2-syntax/src/parser/definition.rs) opens a DependencyDeclaration and reads the
// same parts `dependency_declaration_parts` reads in KerML, so the missing `to` is
// reported at `b` and both names land in the declaration. None of them is the
// Dependency's own:
//
//   PackageMember
//     Dependency "dependency a b;"
//       KwDependency "dependency"
//       DependencyDeclaration "a b"  <- declaration(): both names, and no `to`
//       RelationshipBody ";"         <- body(): the `;`
//   PackageMember
//     Dependency "dependency c to d;"  <- whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding two Dependencies.
//  2. The first's declaration holds `a b` and no `to`, and one diagnostic, on `b`,
//     reported it. The Dependency itself answers no client, no supplier and no `to`: in
//     SysML those are its declaration's, not its own.
//  3. The neighbours still answer. The first's body is its `;`, and the second's
//     declaration is `c to d`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.3 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	dependency a b;
	dependency c to d;
}
