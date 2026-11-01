// Clause: SysML 8.2.2.23 --
//
//   AnalysisCaseDefinition = OccurrenceDefinitionPrefix 'analysis' 'def'
//                            DefinitionDeclaration CaseBody
//   AnalysisCaseUsage      = OccurrenceUsagePrefix 'analysis'
//                            ConstraintUsageDeclaration CaseBody
//
// Both open with `analysis`; only the definition writes `def` after it. This file writes
// the `def` after the name, where neither production has it: `analysis A def B;`, then
// `analysis def C;`. No corpus file writes a `def` after an analysis's name (`grep -rlE
// '\banalysis\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none), so the corpus
// never shows what an `analysis` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_case_definition`
// (crates/sv2-syntax/src/parser/case.rs) decides on the `CASES` keywords FOLLOWED BY `def`
// and on nothing less, so no AnalysisCaseDefinition is begun. `analysis A` is read as an
// AnalysisCaseUsage, whose body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       AnalysisCaseUsage "analysis A "            <- not an AnalysisCaseDefinition
//     Error "def B;"
//     PackageMember
//       AnalysisCaseDefinition "analysis def C;"   <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one AnalysisCaseDefinition, C's.
//  2. A is no AnalysisCaseDefinition, half-built or otherwise: the absence is a missing
//     node, not a definition missing its `def`. Something reported it: one diagnostic, on
//     the `def`.
//  3. The neighbours still answer. The first member's element is an AnalysisCaseUsage,
//     and C's definition is whole, with its own keywords, a declaration of C, and its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.23 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	analysis A def B;
	analysis def C;
}
