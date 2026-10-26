// Clause: SysML 8.2.2.7 --
//
//   AttributeDefinition = DefinitionPrefix 'attribute' 'def' Definition
//   AttributeUsage      = UsagePrefix 'attribute' Usage
//
// Both open with `attribute`; only the definition writes `def` after it. This file
// writes the `def` after the name, where neither production has it: `attribute A def
// B;`, then `attribute def C;`. No corpus file writes a `def` after an attribute's name
// (`grep -rlE '\battribute\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none),
// so the corpus never shows what an `attribute` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_simple_definition`
// (crates/sv2-syntax/src/parser/definition.rs) decides on `attribute` FOLLOWED BY `def`
// and on nothing less, so no AttributeDefinition is begun. `attribute A` is read as an
// AttributeUsage, whose body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       AttributeUsage "attribute A "          <- not an AttributeDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       AttributeDefinition "attribute def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one AttributeDefinition, C's.
//  2. A is no AttributeDefinition, half-built or otherwise: the absence is a missing node,
//     not a definition missing its `def`. Something reported it: one diagnostic, on the
//     `def`.
//  3. The neighbours still answer. The first member's element is an AttributeUsage, and
//     C's definition is whole, with its own keywords and a Definition declaring C.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.7 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	attribute A def B;
	attribute def C;
}
