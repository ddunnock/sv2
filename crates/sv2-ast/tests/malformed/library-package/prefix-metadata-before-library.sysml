// Clause: SysML 8.2.2.5.1 (and KerML 8.2.5.13, which states it identically; one unit,
// .claude/state/grammar/units/LibraryPackage.json) --
//
//   LibraryPackage = ( isStandard ?= 'standard' ) 'library'
//                    ( ownedRelationship += PrefixMetadataMember )*
//                    PackageDeclaration PackageBody
//
// read with deviation LibraryPackage (follow_xtext), which makes the `standard` optional.
// A library package's prefix metadata stands AFTER `library`, where a Package, which has
// no keyword before it, writes its own first. This file writes it in the Package's place:
// `#Approved library package Inner;`. Every one of the 10 library packages in this
// repository's 610 trees writes `library package` and nothing between, so the corpus
// never shows which side of `library` the metadata belongs on. What can go wrong for
// this accessor is ATTRIBUTION: whether metadata written before `library` is the
// package's.
//
// The clause gives that metadata no place in LibraryPackage, so it is not Inner's.
//
// Recovery (NOT a clause, and NOT an ADR): `at_library_package`
// (crates/sv2-syntax/src/parser/namespace.rs) wants `library`, after an optional
// `standard`, at the token it is asked about, and `at_package` looks past the metadata to
// find `package` but finds `library`; so no member starts at the `#`.
// `recover_statement` (crates/sv2-syntax/src/parser/recovery.rs) opens one
// SyntaxKind::Error, takes the first token always, and stops at the member keyword
// `library`, so the Error holds `#Approved` and `library package Inner;` is read as a
// member of its own:
//
//   PackageBody "{ ... }"                       <- Outer's
//     Error "#Approved"                         <- a sibling of the member, not inside it
//     PackageMember "library package Inner;"
//       MemberPrefix ""
//       LibraryPackage "library package Inner;" <- reports NO prefix metadata, not standard
//
// What that pins:
//
//  1. Inner's prefix_metadata() is empty and it is not standard. The metadata written in
//     front of `library` is not its own, and an accessor that looked at preceding text
//     or at a sibling would report metadata the package does not have.
//  2. Something reported it: one diagnostic, spanning exactly the Error.
//  3. The neighbours still answer: Outer, a Package, still owns Inner through its
//     PackageMember, and Outer's body is the one holding the damage.
//
// tests/rejection/library-package-prefix-metadata-follows-library.sysml rejects the same
// misplacement at the top level; this case is the one that asks the accessor about it.
// ADR-0004 supports only the shape -- every token reaches the tree, so this round-trips.
// Do not cite 8.2.2.5.1 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package Outer {
	#Approved library package Inner;
}
