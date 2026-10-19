// Clause: KerML 8.2.3.4.1 --
//
//   NonFeatureMember : OwningMembership =
//       MemberPrefix ownedRelatedElement += MemberElement
//
// and KerML 8.2.5.13 for the head that is cut short here --
//
//   LibraryPackage = ( isStandard ?= 'standard' ) 'library' ... PackageDeclaration ...
//
// KerML only: ADR-0015 scopes this unit to the kerml grammar, and all 608 NonFeatureMember
// nodes in this repository's 610 trees come from .kerml files. Both slots are mandatory,
// and every one of the 608 holds exactly one of each.
//
// What can go wrong for this kind is therefore not a member with an empty slot -- that
// shape is unreachable -- but a member that is never built. That is the contrast with its
// sibling NamespaceFeatureMember, whose dispatch ends in a total fallback and so builds a
// member for anything at all (namespace-feature-member/feature-with-nothing-after-it.kerml).
// This one has no fallback.
//
// Recovery (NOT a clause, and NOT an ADR): `body_element`
// (crates/sv2-syntax/src/parser/namespace.rs) enters `membership` only when
// `at_member_element` holds, and in KerML that means `at_annotating_member` or
// `at_kerml_non_feature_element` (crates/sv2-syntax/src/parser/annotation.rs) -- the
// features having been offered to `kerml_feature_item` first. `kerml_non_feature_element`
// dispatches on the same list the recogniser asks, so once a member is opened its element
// is always read. `standard` with no `library` after it satisfies none of the list, so no
// membership is entered at all: `recover_statement` (crates/sv2-syntax/src/parser/
// recovery.rs) opens one SyntaxKind::Error, takes the first token always, and runs on until
// a member keyword. `class` is one, so the Error closes holding `private standard` and the
// class survives as a member of its own:
//
//   PackageBody
//     Error "private standard"       <- no NonFeatureMember: the visibility is in here
//     NonFeatureMember "class C;"
//       MemberPrefix ""              <- reports nothing; `private` is not its own
//       Class "class C;"
//
// What that pins:
//
//  1. ABSENCE IS THE MEMBER'S, NOT THE SLOT'S. A caller looking for a damaged
//     NonFeatureMember -- one whose element() is None -- finds none, because the
//     damage never reaches this kind. It is an Error beside the members, and reported
//     by exactly one diagnostic spanning exactly that Error.
//  2. THE NEIGHBOURS STILL ANSWER. The package that owns the body is still the root's one
//     member, and `class C;` is still a member owning a Class, with an empty prefix.
//
// ADR-0004 supports only the shape -- every token reaches the tree, so this round-trips.
// Do not cite 8.2.3.4.1 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	private standard
	class C;
}
