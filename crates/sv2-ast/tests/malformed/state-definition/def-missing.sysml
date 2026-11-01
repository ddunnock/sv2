// Clause: SysML 8.2.2.18.1 and 8.2.2.18.2 --
//
//   StateDefinition = OccurrenceDefinitionPrefix 'state' 'def'
//                     DefinitionDeclaration StateDefBody              (8.2.2.18.1)
//   StateUsage      = OccurrenceUsagePrefix 'state'
//                     ActionUsageDeclaration StateUsageBody           (8.2.2.18.2)
//
// Both open with `state`; only the definition writes `def` after it. This file writes the
// `def` after the name, where neither production has it: `state A def B;`, then `state def
// C;`. No corpus file writes a `def` after a state's name (`grep -rlE
// '\bstate\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none), so the corpus
// never shows what a `state` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_state_definition`
// (crates/sv2-syntax/src/parser/state.rs) decides on `state` FOLLOWED BY `def` and on
// nothing less, so no StateDefinition is begun. `state A` is read as a StateUsage, whose
// StateUsageBody is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       StateUsage "state A "          <- not a StateDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       StateDefinition "state def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one StateDefinition, C's.
//  2. A is no StateDefinition, half-built or otherwise: the absence is a missing node,
//     not a definition missing its `def`. Something reported it: one diagnostic, on
//     the `def`.
//  3. The neighbours still answer. The first member's element is a StateUsage, and C's
//     definition is whole, with its own keywords, a declaration of C, and its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.18.1 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	state A def B;
	state def C;
}
