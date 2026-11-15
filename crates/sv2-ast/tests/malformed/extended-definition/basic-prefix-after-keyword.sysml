// Clause: SysML 8.2.2.27 --
//
//   ExtendedDefinition : Definition =
//       BasicDefinitionPrefix? DefinitionExtensionKeyword+ 'def' Definition
//
// and SysML 8.2.2.6.1 --
//
//   BasicDefinitionPrefix = isAbstract ?= 'abstract' | isVariation ?= 'variation'
//
// The basic prefix comes BEFORE the extension keywords. This file writes it after: `#X
// abstract def A;`, then `#X def B;`. No corpus file writes a basic prefix after a `#`
// keyword (`grep -rlE '#[A-Za-z_]+\s+(abstract|variation)\s+def\b'` over the pinned
// corpus finds none), so the corpus never shows what the misplaced word does.
//
// Recovery (NOT a clause, and NOT an ADR): `at_extended_definition`
// (crates/sv2-syntax/src/parser/definition.rs) skips a basic prefix, then the `#`
// keywords, and asks for `def`; it finds `abstract`, so nothing is begun at the `#`. No
// member begins at `abstract`, `def` or `A` either, so `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps the whole statement, up to and
// including its `;`, in an Error, and reports it as unexpected:
//
//   PackageBody
//     Error "#X abstract def A;"        <- the keyword, the prefix, and A: no one's
//     PackageMember
//       ExtendedDefinition "#X def B;"  <- B is whole, with no basic prefix
//
// What that pins:
//
//  1. The parse still yields a tree, holding one ExtendedDefinition, B's.
//  2. The misplaced `abstract` is no definition's basic prefix: A is a missing node, and
//     B, the only definition, answers no basic prefix. Something reported it: one
//     diagnostic, over the Error.
//  3. The neighbours still answer. B is whole, and the package's one member is B's.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.27 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	#X abstract def A;
	#X def B;
}
