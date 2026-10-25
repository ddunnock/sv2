// Clause: SysML 8.2.2.6.1 and 8.2.2.7 --
//
//   DefinitionPrefix : Definition = BasicDefinitionPrefix? DefinitionExtensionKeyword*
//   AttributeDefinition : AttributeDefinition = DefinitionPrefix 'attribute' 'def' Definition
//
// DefinitionPrefix has no `individual`: only an occurrence may be individual, which is
// why OccurrenceDefinitionPrefix is a production of its own (8.2.2.9.1). This file
// writes the slot the production does not have: `individual attribute def A;`. No corpus
// DefinitionPrefix writes anything but `variation` (2) or a `#X` (5), so the corpus never
// shows whether a prefix with no slot for a word claims it anyway.
//
// Recovery (NOT a clause, and NOT an ADR): no member starts at `individual attribute`,
// because an attribute definition takes no `individual`. So `body_element`
// (crates/sv2-syntax/src/parser/namespace.rs) falls through to `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs), which takes `individual` into an Error node
// and stops at `attribute`, a keyword that begins a member. The AttributeDefinition after
// it opens with an EMPTY prefix:
//
//   PackageBody
//     Error "individual"               <- BESIDE the member, not inside it
//     PackageMember
//       AttributeDefinition "attribute def A;"
//         DefinitionPrefix ""          <- zero width: basic_prefix() None, no keywords
//
// What that pins:
//
//  1. The parse still yields a tree, holding one DefinitionPrefix.
//  2. The prefix reports ABSENCE in every slot. It is zero width and claims nothing it has
//     no slot for, and something reported it: one diagnostic, on the `individual`.
//  3. The neighbours still answer. The package body still holds the Error and then one
//     PackageMember, whose element is the AttributeDefinition that owns this prefix.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.6.1 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	individual attribute def A;
}
