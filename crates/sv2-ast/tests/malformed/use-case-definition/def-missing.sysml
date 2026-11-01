// Clause: SysML 8.2.2.25 --
//
//   UseCaseDefinition = OccurrenceDefinitionPrefix 'use' 'case' 'def'
//                       DefinitionDeclaration CaseBody
//   UseCaseUsage      = OccurrenceUsagePrefix 'use' 'case'
//                       ConstraintUsageDeclaration CaseBody
//
// Both open with `use case`; only the definition writes `def` after it. This file writes
// the `def` after the name, where neither production has it: `use case A def B;`, then
// `use case def C;`. No corpus file writes a `def` after a use case's name (`grep -rlE
// '\buse\s+case\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none), so the
// corpus never shows what a `use case` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_case_definition`
// (crates/sv2-syntax/src/parser/case.rs) decides on all of a `CASES` row's keywords
// FOLLOWED BY `def` and on nothing less, so no UseCaseDefinition is begun. `use case A`
// is read as a UseCaseUsage, whose body is missing and reported on the `def`;
// `recover_statement` (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an
// Error beside it, and `emit` keeps only the first of the two diagnostics that start
// there:
//
//   PackageBody
//     PackageMember
//       UseCaseUsage "use case A "            <- not a UseCaseDefinition
//     Error "def B;"
//     PackageMember
//       UseCaseDefinition "use case def C;"   <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one UseCaseDefinition, C's.
//  2. A is no UseCaseDefinition, half-built or otherwise: the absence is a missing node,
//     not a definition missing its `def`. Something reported it: one diagnostic, on the
//     `def`.
//  3. The neighbours still answer. The first member's element is a UseCaseUsage, and C's
//     definition is whole, with its own three keywords, a declaration of C, and its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.25 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	use case A def B;
	use case def C;
}
