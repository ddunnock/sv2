// Clause: KerML 8.2.3.4.2 and SysML 8.2.2.5.1 (one unit, shared) --
//
//   NamespaceImport = importedNamespace = [QualifiedName] '::' '*' ( '::' isRecursive ?= '**' )?
//                   | importedNamespace = FilterPackage { ownedRelatedElement += importedNamespace }
//
// The recursive suffix is optional, but once its `::` is written the `**` is not. This
// file writes the `::` and stops: `public import A::*::;`. Every first-alternative
// NamespaceImport in the corpus either writes the whole suffix (2) or none of it (468);
// the other 11 are the FilterPackage alternative, which has no suffix. So the corpus never
// shows a suffix begun and not finished. That is the shape in which "a second `::` is
// written" and "isRecursive is set" come apart.
//
// Recovery (NOT a clause, and NOT an ADR): `import_suffix`
// (crates/sv2-syntax/src/parser/import.rs) takes `::` and `*` and calls `recursive_suffix`,
// which takes the second `::` and `expect`s a `**`. It reports the `;` it finds instead
// and consumes nothing, so the `;` stays the RelationshipBody's, and `imported` closes the
// checkpoint as a NamespaceImport holding the dangling `::`:
//
//   Import "public import A::*::;"
//     VisibilityIndicator "public"
//     ImportDeclaration "A::*::"
//       NamespaceImport "A::*::"      <- is_recursive(): NO `**`, so false
//         QualifiedName "A"           <- name(): still `A`
//         '::' '*' '::'               <- its own tokens; the last one is the dangling one
//     RelationshipBody ";"
//
// What that pins:
//
//  1. The parse still yields a tree, holding one NamespaceImport and no FilterPackage.
//  2. is_recursive() reports ABSENCE. It does not guess from the second `::`, and
//     something reported it: one diagnostic, at the `;` where the `**` should be.
//  3. The neighbours still answer. name() is still `A`, untouched by the broken suffix,
//     and the Import still holds this declaration, its visibility and its `;` body.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.3.4.2 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	public import A::*::;
}
