// Clause: SysML 8.2.2.20 --
//
//   ConstraintDefinition = OccurrenceDefinitionPrefix 'constraint' 'def'
//                          DefinitionDeclaration CalculationBody
//   ConstraintUsage      = OccurrenceUsagePrefix 'constraint'
//                          ConstraintUsageDeclaration CalculationBody
//
// Both open with `constraint`; only the definition writes `def` after it. This file
// writes the `def` after the name, where neither production has it: `constraint A def
// B;`, then `constraint def C;`. No corpus file writes a `def` after a constraint's name
// (`grep -rlE '\bconstraint\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none),
// so the corpus never shows what a `constraint` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_constraint_definition`
// (crates/sv2-syntax/src/parser/calculation.rs) decides on `constraint` FOLLOWED BY `def`
// and on nothing less, so no ConstraintDefinition is begun. `constraint A` is read as a
// ConstraintUsage, whose body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       ConstraintUsage "constraint A "          <- not a ConstraintDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       ConstraintDefinition "constraint def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one ConstraintDefinition, C's.
//  2. A is no ConstraintDefinition, half-built or otherwise: the absence is a missing
//     node, not a definition missing its `def`. Something reported it: one diagnostic, on
//     the `def`.
//  3. The neighbours still answer. The first member's element is a ConstraintUsage, and
//     C's definition is whole, with its own keywords, a declaration of C, and its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.20 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	constraint A def B;
	constraint def C;
}
