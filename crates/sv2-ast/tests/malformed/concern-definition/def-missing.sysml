// Clause: SysML 8.2.2.21.3 --
//
//   ConcernDefinition = OccurrenceDefinitionPrefix 'concern' 'def'
//                       DefinitionDeclaration RequirementBody
//   ConcernUsage      = OccurrenceUsagePrefix 'concern'
//                       ConstraintUsageDeclaration RequirementBody
//
// Both open with `concern`; only the definition writes `def` after it. This file writes
// the `def` after the name, where neither production has it: `concern A def B;`, then
// `concern def C;`. No corpus file writes a `def` after a concern's name (`grep -rlE
// '\bconcern\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none), so the corpus
// never shows what a `concern` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_concern_definition`
// (crates/sv2-syntax/src/parser/requirement.rs) decides on `concern` FOLLOWED BY `def`
// and on nothing less, so no ConcernDefinition is begun. `concern A` is read as a
// ConcernUsage, whose body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       ConcernUsage "concern A "          <- not a ConcernDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       ConcernDefinition "concern def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one ConcernDefinition, C's.
//  2. A is no ConcernDefinition, half-built or otherwise: the absence is a missing
//     node, not a definition missing its `def`. Something reported it: one diagnostic, on
//     the `def`.
//  3. The neighbours still answer. The first member's element is a ConcernUsage, and
//     C's definition is whole, with its own keywords, a declaration of C, and its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.21.3 for the recovery outcome, and do not cite ADR-0022, which is
// about deviation diagnostics and says nothing about recovery.
package P {
	concern A def B;
	concern def C;
}
