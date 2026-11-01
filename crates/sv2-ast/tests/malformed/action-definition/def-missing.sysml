// Clause: SysML 8.2.2.17.1 --
//
//   ActionDefinition = OccurrenceDefinitionPrefix 'action' 'def'
//                      DefinitionDeclaration ActionBody
//   ActionUsage      = OccurrenceUsagePrefix 'action'
//                      ActionUsageDeclaration ActionBody
//
// Both open with `action`; only the definition writes `def` after it. This file writes the
// `def` after the name, where neither production has it: `action A def B;`, then `action def
// C;`. No corpus file writes a `def` after an action's name (`grep -rlE
// '\baction\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none), so the corpus
// never shows what an `action` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_action_definition`
// (crates/sv2-syntax/src/parser/action.rs) decides on `action` FOLLOWED BY `def` and
// on nothing less, so no ActionDefinition is begun. `action A` is read as a
// ActionUsage, whose body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       ActionUsage "action A "          <- not an ActionDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       ActionDefinition "action def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one ActionDefinition, C's.
//  2. A is no ActionDefinition, half-built or otherwise: the absence is a missing
//     node, not a definition missing its `def`. Something reported it: one diagnostic, on
//     the `def`.
//  3. The neighbours still answer. The first member's element is an ActionUsage, and
//     C's definition is whole, with its own keywords, a declaration of C, and its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.17.1 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	action A def B;
	action def C;
}
