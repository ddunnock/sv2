// Clause: SysML 8.2.2.26.1 and 8.2.2.26.2 --
//
//   ViewDefinition = OccurrenceDefinitionPrefix 'view' 'def'
//                    DefinitionDeclaration ViewDefinitionBody
//   ViewUsage      = OccurrenceUsagePrefix 'view'
//                    UsageDeclaration? ValuePart? ViewBody
//
// Both open with `view`; only the definition writes `def` after it. This file writes the
// `def` after the name, where neither production has it: `view A def B;`, then `view def
// C;`. No corpus file writes a `def` after a view's name (`grep -rlE
// '\bview\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none), so the corpus
// never shows what a `view` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_view_definition`
// (crates/sv2-syntax/src/parser/view.rs) decides on `view` FOLLOWED BY `def` and on
// nothing less, so no ViewDefinition is begun. `view A` is read as a ViewUsage, whose
// body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       ViewUsage "view A "          <- not a ViewDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       ViewDefinition "view def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one ViewDefinition, C's.
//  2. A is no ViewDefinition, half-built or otherwise: the absence is a missing node, not
//     a definition missing its `def`. Something reported it: one diagnostic, on the
//     `def`.
//  3. The neighbours still answer. The first member's element is a ViewUsage, and C's
//     definition is whole, with its own keywords, a declaration of C, and its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.26.1 for the recovery outcome, and do not cite ADR-0022, which is
// about deviation diagnostics and says nothing about recovery.
package P {
	view A def B;
	view def C;
}
