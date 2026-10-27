// Clause: SysML 8.2.2.12 --
//
//   PortDefinition = DefinitionPrefix 'port' 'def' Definition
//                    ownedRelationship += ConjugatedPortDefinitionMember
//   PortUsage      = OccurrenceUsagePrefix 'port' Usage
//
// Both open with `port`; only the definition writes `def` after it. This file writes the
// `def` after the name, where neither production has it: `port A def B;`, then `port def
// C;`. No corpus file writes a `def` after a port's name (`grep -rlE
// '\bport\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b' vendor/corpus` finds none), so the corpus
// never shows what a `port` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_port_definition`
// (crates/sv2-syntax/src/parser/connection.rs) decides on `port` FOLLOWED BY `def` and on
// nothing less, so no PortDefinition is begun. `port A` is read as a PortUsage, whose
// body is missing and reported on the `def`; `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an Error beside it, and
// `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       PortUsage "port A "          <- not a PortDefinition: cast() refuses it
//     Error "def B;"
//     PackageMember
//       PortDefinition "port def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one PortDefinition, C's.
//  2. A is no PortDefinition, half-built or otherwise: the absence is a missing node, not
//     a definition missing its `def`. Something reported it: one diagnostic, on the `def`.
//  3. The neighbours still answer. The first member's element is a PortUsage, and C's
//     definition is whole, with its own keywords and a Definition declaring C.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.12 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	port A def B;
	port def C;
}
