// Clause: SysML 8.2.2.26.3 --
//
//   ViewpointDefinition = OccurrenceDefinitionPrefix 'viewpoint' 'def'
//                         DefinitionDeclaration RequirementBody
//   ViewpointUsage      = OccurrenceUsagePrefix 'viewpoint'
//                         ConstraintUsageDeclaration RequirementBody
//
// Both open with `viewpoint`; only the definition writes `def` after it. This file writes
// the `def` after the name, where neither production has it: `viewpoint A def B;`, then
// `viewpoint def C;`. No corpus file writes a `def` after a viewpoint's name (`grep -rlE
// '\bviewpoint\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none), so the
// corpus never shows what a `viewpoint` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_viewpoint_definition`
// (crates/sv2-syntax/src/parser/view.rs) decides on `viewpoint` FOLLOWED BY `def` and on
// nothing less, so no ViewpointDefinition is begun. `viewpoint A` is read as a
// ViewpointUsage, whose body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       ViewpointUsage "viewpoint A "          <- not a ViewpointDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       ViewpointDefinition "viewpoint def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one ViewpointDefinition, C's.
//  2. A is no ViewpointDefinition, half-built or otherwise: the absence is a missing
//     node, not a definition missing its `def`. Something reported it: one diagnostic, on
//     the `def`.
//  3. The neighbours still answer. The first member's element is a ViewpointUsage, and
//     C's definition is whole, with its own keywords, a declaration of C, and its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.26.3 for the recovery outcome, and do not cite ADR-0022, which is
// about deviation diagnostics and says nothing about recovery.
package P {
	viewpoint A def B;
	viewpoint def C;
}
