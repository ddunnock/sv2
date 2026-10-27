// Clause: SysML 8.2.2.10 --
//
//   ItemDefinition = OccurrenceDefinitionPrefix 'item' 'def' Definition
//   ItemUsage      = OccurrenceUsagePrefix 'item' Usage
//
// Both open with `item`; only the definition writes `def` after it. This file writes the
// `def` after the name, where neither production has it: `item A def B;`, then `item def
// C;`. No corpus file writes a `def` after an item's name (`grep -rlE
// '\bitem\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none), so the corpus
// never shows what an `item` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_simple_definition`
// (crates/sv2-syntax/src/parser/definition.rs) decides on `item` FOLLOWED BY `def` and on
// nothing less, so no ItemDefinition is begun. `item A` is read as an ItemUsage, whose
// body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       ItemUsage "item A "          <- not an ItemDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       ItemDefinition "item def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one ItemDefinition, C's.
//  2. A is no ItemDefinition, half-built or otherwise: the absence is a missing node, not
//     a definition missing its `def`. Something reported it: one diagnostic, on the `def`.
//  3. The neighbours still answer. The first member's element is an ItemUsage, and C's
//     definition is whole, with its own keywords and a Definition declaring C.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.10 for the recovery outcome, and do not cite ADR-0022, which is
// about deviation diagnostics and says nothing about recovery.
package P {
	item A def B;
	item def C;
}
