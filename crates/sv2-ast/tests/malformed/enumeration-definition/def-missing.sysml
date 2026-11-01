// Clause: SysML 8.2.2.8 --
//
//   EnumerationDefinition = DefinitionExtensionKeyword* 'enum' 'def'
//                           DefinitionDeclaration EnumerationBody
//   EnumerationUsage      = UsagePrefix 'enum' Usage
//
// Both open with `enum`; only the definition writes `def` after it. This file writes the
// `def` after the name, where neither production has it: `enum A def B;`, then `enum def
// C;`. No corpus file writes a `def` after an enum's name (`grep -rlE
// '\benum\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b'` over the pinned corpus finds none), so the
// corpus never shows what an `enum` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_enumeration_definition`
// (crates/sv2-syntax/src/parser/definition.rs) decides on `enum` FOLLOWED BY `def` and on
// nothing less, so no EnumerationDefinition is begun. `enum A` is read as an
// EnumerationUsage, whose body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       EnumerationUsage "enum A "            <- not an EnumerationDefinition
//     Error "def B;"
//     PackageMember
//       EnumerationDefinition "enum def C;"   <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one EnumerationDefinition, C's.
//  2. A is no EnumerationDefinition, half-built or otherwise: the absence is a missing
//     node, not a definition missing its `def`. Something reported it: one diagnostic, on
//     the `def`.
//  3. The neighbours still answer. The first member's element is an EnumerationUsage, and
//     C's definition is whole, with its own keywords, a declaration of C, and its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.8 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	enum A def B;
	enum def C;
}
