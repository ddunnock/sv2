// Clause: SysML 8.2.2.21.1 and 8.2.2.21.2 --
//
//   RequirementDefinition = OccurrenceDefinitionPrefix 'requirement' 'def'
//                           DefinitionDeclaration RequirementBody
//   RequirementUsage      = OccurrenceUsagePrefix 'requirement'
//                           ConstraintUsageDeclaration RequirementBody
//
// Both open with `requirement`; only the definition writes `def` after it. This file
// writes the `def` after the name, where neither production has it: `requirement A def
// B;`, then `requirement def C;`. No corpus file writes a `def` after a requirement's name
// (`grep -rlE '\brequirement\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds
// none), so the corpus never shows what a `requirement` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_requirement_definition`
// (crates/sv2-syntax/src/parser/requirement.rs) decides on `requirement` FOLLOWED BY `def`
// and on nothing less, so no RequirementDefinition is begun. `requirement A` is read as a
// RequirementUsage, whose body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       RequirementUsage "requirement A "           <- not a RequirementDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       RequirementDefinition "requirement def C;"  <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one RequirementDefinition, C's.
//  2. A is no RequirementDefinition, half-built or otherwise: the absence is a missing
//     node, not a definition missing its `def`. Something reported it: one diagnostic, on
//     the `def`.
//  3. The neighbours still answer. The first member's element is a RequirementUsage, and
//     C's definition is whole, with its own keywords, a declaration of C, and its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.21.1 for the recovery outcome, and do not cite ADR-0022, which is
// about deviation diagnostics and says nothing about recovery.
package P {
	requirement A def B;
	requirement def C;
}
