// Clause: SysML 8.2.2.16 --
//
//   FlowDefinition = OccurrenceDefinitionPrefix 'flow' 'def' Definition
//   FlowUsage      = OccurrenceUsagePrefix 'flow' FlowDeclaration DefinitionBody
//
// Both can open with `flow`; only the definition writes `def` after it. This file writes
// the `def` after the name, where neither production has it: `flow A def B;`, then `flow
// def C;`. No corpus file writes a `def` after a flow's name (`grep -rlE
// '\bflow\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none), so the corpus
// never shows what a `flow` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_simple_definition`
// (crates/sv2-syntax/src/parser/definition.rs) decides on `flow` FOLLOWED BY `def` and on
// nothing less, so no FlowDefinition is begun. `flow A` is read as a FlowUsage, whose
// body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       FlowUsage "flow A "          <- not a FlowDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       FlowDefinition "flow def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one FlowDefinition, C's.
//  2. A is no FlowDefinition, half-built or otherwise: the absence is a missing node, not
//     a definition missing its `def`. Something reported it: one diagnostic, on the `def`.
//  3. The neighbours still answer. The first member's element is a FlowUsage, and C's
//     definition is whole, with its own keywords and a Definition declaring C.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.16 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	flow A def B;
	flow def C;
}
