// Clause: SysML 8.2.2.7 and 8.2.2.6.1 --
//
//   AttributeDefinition = DefinitionPrefix 'attribute' 'def' Definition
//   DefinitionPrefix    = BasicDefinitionPrefix? DefinitionExtensionKeyword*
//
// An attribute definition opens with a DefinitionPrefix, and a DefinitionPrefix has no
// `individual`: that group belongs to OccurrenceDefinitionPrefix (8.2.2.9.1), and an
// attribute is not an occurrence. It is the one condition that separates
// AttributeDefinition from the seven siblings `simple_definition` builds with it, all of
// which take the occurrence prefix. This file writes `individual attribute def A;`, then
// `attribute def B;`. No corpus file writes `individual attribute` (`grep -rlE
// '\bindividual\s+attribute\b' vendor/corpus` finds none).
//
// Recovery (NOT a clause, and NOT an ADR): the SIMPLE_DEFINITIONS entry for `attribute`
// in crates/sv2-syntax/src/parser/definition.rs has `is_occurrence: false`, so
// `at_simple_definition` skips a DefinitionPrefix, not an occurrence one, and sees no
// definition at `individual`. Nothing else begins a member there, so `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps the lone `individual` in an Error,
// and the definition after it is read whole, from `attribute`:
//
//   PackageBody
//     Error "individual"                       <- not the prefix's
//     PackageMember
//       AttributeDefinition "attribute def A;"
//         DefinitionPrefix ""                  <- prefix(): present, empty
//         KwAttribute "attribute"
//         KwDef "def"
//         Definition "A;"
//     PackageMember
//       AttributeDefinition "attribute def B;"
//
// What that pins:
//
//  1. The parse still yields a tree, holding two AttributeDefinitions.
//  2. A's definition starts at `attribute`, not at `individual`, and its prefix is empty:
//     the `individual` is absent from it rather than absorbed. Something reported it: one
//     diagnostic, on the `individual`.
//  3. The neighbours still answer. A and B are both whole, each with its keywords and a
//     Definition declaring it, and DefinitionPrefix answers absence for both of A's slots.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.7 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	individual attribute def A;
	attribute def B;
}
