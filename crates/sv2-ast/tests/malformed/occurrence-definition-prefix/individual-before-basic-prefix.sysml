// Clause: SysML 8.2.2.9.1 --
//
//   OccurrenceDefinitionPrefix : OccurrenceDefinition =
//       BasicDefinitionPrefix?
//       ( isIndividual ?= 'individual' ownedRelationship += EmptyMultiplicityMember )?
//       DefinitionExtensionKeyword*
//
// The slots are ordered: `individual` comes after the BasicDefinitionPrefix, never before
// it. This file writes them the wrong way round: `individual abstract part def X;`. In the
// corpus no prefix writes two slots at all (1,020 prefixes: 970 empty, 23 `abstract` or
// `variation`, 24 `individual`, 3 `#X`), so it never shows what happens to a slot written
// out of order: whether the prefix claims it anyway, or it is lost.
//
// Recovery (NOT a clause, and NOT an ADR): no member starts at `individual` followed by
// `abstract`, so `body_element` (crates/sv2-syntax/src/parser/namespace.rs) falls through
// to `recover_statement` (crates/sv2-syntax/src/parser/recovery.rs). That takes
// `individual` into an Error node, reports it, and stops at `abstract` because a keyword
// that begins a member ends the statement. The PartDefinition after it reads `abstract`
// as its basic prefix:
//
//   PackageBody
//     Error "individual"               <- BESIDE the member, not inside it
//     PackageMember
//       PartDefinition "abstract part def X;"
//         OccurrenceDefinitionPrefix "abstract"
//           BasicDefinitionPrefix "abstract"   <- basic_prefix(): still `abstract`
//                                              <- is_individual(): false
//                                              <- multiplicity(): None
//
// What that pins:
//
//  1. The parse still yields a tree, holding one PartDefinition and one prefix.
//  2. The prefix reports ABSENCE of `individual` rather than claiming a keyword written
//     outside it, and that absence carries no EmptyMultiplicityMember either, since the
//     production writes the two together. Something reported it: one diagnostic, on the
//     `individual` itself.
//  3. The neighbours still answer. basic_prefix() is still `abstract`, and the Package
//     still owns one PackageMember whose element is the PartDefinition, beside the Error.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.9.1 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	individual abstract part def X;
}
