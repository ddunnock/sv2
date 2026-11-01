// Clause: SysML 8.2.2.8 --
//
//   EnumerationBody = ';'
//                   | '{' ( AnnotatingMember | EnumerationUsageMember )* '}'
//
// An enumeration body's items are annotations and enumerated values, and nothing else:
// "any owned members declared in the body of an enumeration definition must be
// enumeration usages" (7.8.2, cited at `enumeration_body`). This file writes an
// enumeration definition inside one: `enum def E { enum def F; }`. No corpus file nests
// one enumeration definition in another, so the corpus never shows what becomes of it.
//
// Recovery (NOT a clause, and NOT an ADR): `enumeration_body_items`
// (crates/sv2-syntax/src/parser/definition.rs) has its own item loop rather than
// `body_elements`, and `at_enumerated_value` refuses an `enum` followed by `def`, so no
// item begins at `enum def F;`. `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps the whole statement in an Error inside
// the body, its `enum` and `def` no keywords but names, and reports it once, as
// unexpected:
//
//   PackageBody
//     PackageMember
//       EnumerationDefinition "enum def E { enum def F; }"
//         KwEnum, KwDef, DefinitionDeclaration "E"
//         EnumerationBody "{ enum def F; }"
//           Error "enum def F;"              <- not an EnumerationDefinition, nor any item
//
// What that pins:
//
//  1. The parse still yields a tree, holding one EnumerationDefinition, E's.
//  2. F is no EnumerationDefinition: a definition is no item of an enumeration body, and
//     the body holds nothing for it but an Error. Something reported it: one diagnostic,
//     over the Error.
//  3. The neighbours still answer. E is whole: its own keywords, a declaration of E, and
//     a body that opens with `{` and closes with `}` around the Error.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.8 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	enum def E { enum def F; }
}
