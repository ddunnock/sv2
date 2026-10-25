// Clause: SysML 8.2.2.6.1 --
//
//   Definition = DefinitionDeclaration DefinitionBody
//   DefinitionBody : Type = ';' | '{' DefinitionBodyItem* '}'
//
// The body is not optional: a definition ends in `;` or a braced body. This file leaves
// it out and starts the next member straight away: `part def A` and then `part def B;`.
// Every one of the corpus's 1,064 definitions writes its body, so the corpus never shows
// a definition whose body slot is present in the tree and holds nothing, nor whether the
// next member survives being written where the body should be.
//
// Recovery (NOT a clause, and NOT an ADR): `definition_body`
// (crates/sv2-syntax/src/parser/definition.rs) opens its node before it looks for `;` or
// `{`, finds neither, and reports the `part` it finds instead with `error_expected`,
// consuming nothing. So the body closes empty, and the PartDefinition for B is read as
// the next member:
//
//   PackageBody
//     PackageMember
//       PartDefinition "part def A"
//         Definition "A"
//           DefinitionDeclaration "A"    <- declaration(): A, untouched
//           DefinitionBody ""            <- body(): PRESENT, zero width, no `;` or `{`
//     PackageMember
//       PartDefinition "part def B;"     <- B is its own definition, with its own `;`
//
// What that pins:
//
//  1. The parse still yields a tree, holding two Definitions.
//  2. A's body() is present but empty, so present is not evidence that a body was
//     written, and something reported it: one diagnostic, on the `part` found where A's
//     body should be.
//  3. The neighbours still answer. A's declaration() still names A, and B's Definition
//     is whole: its declaration names B and its body is the `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.6.1 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	part def A
	part def B;
}
