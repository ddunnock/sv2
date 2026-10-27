// Clause: SysML 8.2.2.9.1 --
//
//   IndividualDefinition : OccurrenceDefinition =
//       BasicDefinitionPrefix? isIndividual ?= 'individual'
//       DefinitionExtensionKeyword* 'def' Definition
//       ownedRelationship += EmptyMultiplicityMember
//
// The basic prefix, `abstract` or `variation`, stands BEFORE `individual`. This file
// writes it after: `individual abstract def A;`, then `individual def B;`. No corpus file
// writes `individual abstract` or `individual variation` (`grep -rlE
// '\bindividual\s+(abstract|variation)\b' vendor/corpus` finds none), so the corpus never
// shows what the reversed order becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_individual_definition`
// (crates/sv2-syntax/src/parser/definition.rs) asks for `def` after `individual` and its
// `#` keywords, and finds `abstract`, so no IndividualDefinition is begun; nothing else
// begins at `individual abstract` either. `body_elements`
// (crates/sv2-syntax/src/parser/namespace.rs) falls through to `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs), which takes the statement up to and
// including its `;` into one Error and reports it once, on that whole run:
//
//   PackageBody
//     Error "individual abstract def A;"       <- no member, and no IndividualDefinition
//     PackageMember
//       IndividualDefinition "individual def B;" <- B is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one IndividualDefinition, B's.
//  2. A is no IndividualDefinition, half-built or otherwise, and its `abstract` is no
//     one's basic prefix: the absence is a missing node. Something reported it: one
//     diagnostic, over the whole Error.
//  3. The neighbours still answer. The package's members are the package itself and B's
//     alone, and B is whole, with no basic prefix, its own keywords, and a Definition
//     declaring B.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.9.1 for the recovery outcome, and do not cite ADR-0022, which is
// about deviation diagnostics and says nothing about recovery.
package P {
	individual abstract def A;
	individual def B;
}
