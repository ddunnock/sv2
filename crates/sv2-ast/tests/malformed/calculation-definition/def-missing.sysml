// Clause: SysML 8.2.2.19 --
//
//   CalculationDefinition = OccurrenceDefinitionPrefix 'calc' 'def'
//                           DefinitionDeclaration CalculationBody
//   CalculationUsage      = OccurrenceUsagePrefix 'calc'
//                           ActionUsageDeclaration CalculationBody
//
// Both open with `calc`; only the definition writes `def` after it. This file writes the
// `def` after the name, where neither production has it: `calc A def B;`, then `calc def
// C;`. No corpus file writes a `def` after a calc's name (`grep -rlE
// '\bcalc\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none), so the corpus
// never shows what a `calc` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_calculation_definition`
// (crates/sv2-syntax/src/parser/calculation.rs) decides on `calc` FOLLOWED BY `def` and
// on nothing less, so no CalculationDefinition is begun. `calc A` is read as a
// CalculationUsage, whose body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       CalculationUsage "calc A "          <- not a CalculationDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       CalculationDefinition "calc def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one CalculationDefinition, C's.
//  2. A is no CalculationDefinition, half-built or otherwise: the absence is a missing
//     node, not a definition missing its `def`. Something reported it: one diagnostic, on
//     the `def`.
//  3. The neighbours still answer. The first member's element is a CalculationUsage, and
//     C's definition is whole, with its own keywords, a declaration of C, and its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.19 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	calc A def B;
	calc def C;
}
