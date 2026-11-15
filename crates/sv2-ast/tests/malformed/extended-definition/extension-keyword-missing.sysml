// Clause: SysML 8.2.2.27 --
//
//   ExtendedDefinition : Definition =
//       BasicDefinitionPrefix? DefinitionExtensionKeyword+ 'def' Definition
//
// ONE OR MORE extension keywords: with no kind keyword, the `#X` is the only thing that
// says what is being defined ("A user-defined keyword for semantic metadata may also be
// used to declare a definition or usage without using any language-defined keyword",
// 7.27.4). This file writes `def` with none: `def A;`, then `#X def B;`. No corpus file
// opens a line with `def` (`grep -rnE '^\s*def\s'` over the pinned corpus finds none), so
// the corpus never shows what a bare `def` becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_extended_definition`
// (crates/sv2-syntax/src/parser/definition.rs) asks for at least one `#` keyword before the
// `def` and finds none, and no other member begins at `def`. `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) wraps the statement, up to and including its
// `;`, in an Error, and reports it as unexpected:
//
//   PackageBody
//     Error "def A;"                    <- not an ExtendedDefinition, half-built or otherwise
//     PackageMember
//       ExtendedDefinition "#X def B;"  <- B is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one ExtendedDefinition, B's.
//  2. A is no ExtendedDefinition: a definition with no keyword is a missing node, never
//     a node whose extension_keywords() is empty. Something reported it: one diagnostic,
//     over the Error.
//  3. The neighbours still answer. B is whole, and the package's one member is B's.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.27 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	def A;
	#X def B;
}
