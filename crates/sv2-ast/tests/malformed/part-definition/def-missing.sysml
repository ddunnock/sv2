// Clause: SysML 8.2.2.11 --
//
//   PartDefinition = OccurrenceDefinitionPrefix 'part' 'def' Definition
//   PartUsage      = OccurrenceUsagePrefix 'part' Usage
//
// Both open with `part`; only the definition writes `def` after it. This file writes the
// `def` after the name, where neither production has it: `part A def B;`, then
// `part def C;`. No corpus file writes a `def` after a part's name (`grep -rlE
// '\bpart\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none), so the corpus
// never shows what a `part` with no `def` after it becomes. It matters beyond this kind:
// the seven definitions `simple_definition` shares this method with differ from
// PartDefinition only in that first keyword.
//
// Recovery (NOT a clause, and NOT an ADR): `at_simple_definition`
// (crates/sv2-syntax/src/parser/definition.rs) decides on `part` FOLLOWED BY `def` and
// on nothing less, so no PartDefinition is begun. `part A` is read as a PartUsage, whose
// body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       PartUsage "part A"               <- not a PartDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       PartDefinition "part def C;"     <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one PartDefinition, C's.
//  2. A is no PartDefinition, half-built or otherwise: the absence is a missing node, not
//     a definition missing its `def`. Something reported it: one diagnostic, on the `def`.
//  3. The neighbours still answer. The first member's element is a PartUsage, and C's
//     definition is whole, with its own keywords and a Definition declaring C.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.11 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	part A def B;
	part def C;
}
