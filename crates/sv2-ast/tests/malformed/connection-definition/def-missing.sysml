// Clause: SysML 8.2.2.13.1 --
//
//   ConnectionDefinition = OccurrenceDefinitionPrefix 'connection' 'def' Definition
//   ConnectionUsage      = OccurrenceUsagePrefix
//                            ( 'connection' UsageDeclaration ValuePart?
//                              ( 'connect' ConnectorPart )?
//                            | 'connect' ConnectorPart ) UsageBody
//
// Both can open with `connection`; only the definition writes `def` after it. This file
// writes the `def` after the name, where neither production has it: `connection A def
// B;`, then `connection def C;`. No corpus file writes a `def` after a connection's name
// (`grep -rlE '\bconnection\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none),
// so the corpus never shows what a `connection` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_simple_definition`
// (crates/sv2-syntax/src/parser/definition.rs) decides on `connection` FOLLOWED BY `def`
// and on nothing less, so no ConnectionDefinition is begun. `connection A` is read as a
// ConnectionUsage, whose body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       ConnectionUsage "connection A "          <- not a ConnectionDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       ConnectionDefinition "connection def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one ConnectionDefinition, C's.
//  2. A is no ConnectionDefinition, half-built or otherwise: the absence is a missing
//     node, not a definition missing its `def`. Something reported it: one diagnostic, on
//     the `def`.
//  3. The neighbours still answer. The first member's element is a ConnectionUsage, and
//     C's definition is whole, with its own keywords and a Definition declaring C.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.13.1 for the recovery outcome, and do not cite ADR-0022, which is
// about deviation diagnostics and says nothing about recovery.
package P {
	connection A def B;
	connection def C;
}
