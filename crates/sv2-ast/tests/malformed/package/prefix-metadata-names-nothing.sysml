// Clause: SysML 8.2.2.5.1 (and KerML 8.2.5.13, which states it identically; one unit,
// .claude/state/grammar/units/Package.json) --
//
//   Package = ( ownedRelationship += PrefixMetadataMember )*
//             PackageDeclaration PackageBody
//
// The declaration and the body are mandatory, and every one of the 666 Package nodes in
// this repository's 610 trees holds exactly one of each and nothing else -- so this kind's
// own shape cannot be wrong there. The one slot that can be written wrongly is the
// optional prefix metadata, and no corpus file writes it at all. What can go wrong for
// this accessor is therefore ATTRIBUTION: whether a `#` written in front of a package is
// that package's.
//
// The clause makes a PrefixMetadataMember a `#` followed by the metadata it names. A bare
// `#` names nothing, so it is not one, and the clause says nothing about where it goes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_package`
// (crates/sv2-syntax/src/parser/namespace.rs) looks past PrefixMetadataMembers with
// `skip_prefix_metadata` to find `package`, and a `#` with no name after it is not one to
// look past, so no package starts at the `#`. `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) opens one SyntaxKind::Error, takes the first
// token always, and stops at the member keyword `package`, so the Error holds the `#`
// alone and `package Inner;` is read as a member of its own:
//
//   PackageBody "{ ... }"            <- Outer's
//     Error "#"                      <- a sibling of the member, not inside it
//     PackageMember "package Inner;"
//       MemberPrefix ""
//       Package "package Inner;"     <- reports NO prefix metadata
//
// What that pins:
//
//  1. Inner's prefix_metadata() is empty. The `#` written in front of it is not its
//     metadata, and an accessor that looked at preceding text or at a sibling would
//     report one the package does not have.
//  2. Something reported it: one diagnostic, spanning exactly the Error.
//  3. The neighbours still answer: Outer still owns Inner through its PackageMember, and
//     Outer's own slots are unharmed by damage inside its body.
//
// ADR-0004 supports only the shape -- every token reaches the tree, so this round-trips.
// Do not cite 8.2.2.5.1 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package Outer {
	# package Inner;
}
