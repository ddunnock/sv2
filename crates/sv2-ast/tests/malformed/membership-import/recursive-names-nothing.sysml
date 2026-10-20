// Clause: KerML 8.2.3.4.2 and SysML 8.2.2.5.1 (one unit, shared) --
//
//   MembershipImport = importedMembership = [QualifiedName] ( '::' isRecursive ?= '**' )?
//
// The QualifiedName is mandatory and the recursive suffix optional. This file writes the
// suffix and leaves out the name: `public import ::**;`. In this repository's 610 trees
// every one of the 274 corpus MembershipImports names something, so the corpus never
// shows the two slots answering independently -- whether a missing name takes the
// recursion with it, or a written `**` stands in for a name.
//
// Recovery (NOT a clause, and NOT an ADR): `qualified_name`
// (crates/sv2-syntax/src/parser/import.rs) opens its node and asks `expect_name`
// (crates/sv2-syntax/src/parser/recovery.rs) for a name; `error_expected` reports the
// `::` it found instead and consumes nothing, and the loop takes no separator because no
// name follows it. So the name closes empty, `import_suffix` takes `::` and `**` as the
// recursive suffix, and `imported` closes the checkpoint as a MembershipImport:
//
//   Import "public import ::**;"
//     VisibilityIndicator "public"
//     ImportDeclaration "::**"
//       MembershipImport "::**"        <- is_recursive(): the `**` is written, and its own
//         QualifiedName ""             <- zero width, NO segments
//     RelationshipBody ";"
//
// What that pins:
//
//  1. name() is Some and reports NO segments: present is not evidence that anything was
//     named, as for every QualifiedName this parser opens before seeking a name.
//  2. Something reported it: one diagnostic, at the `::` where the name should be.
//  3. is_recursive() still answers what was written, which is the recursive suffix: the
//     two slots are independent, and the missing name does not take the `**` with it.
//     The neighbours answer too: the Import still holds this declaration, its visibility
//     and its `;` body.
//
// ADR-0004 supports only the shape -- every byte reaches the tree, so this round-trips.
// Do not cite 8.2.3.4.2 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	public import ::**;
}
