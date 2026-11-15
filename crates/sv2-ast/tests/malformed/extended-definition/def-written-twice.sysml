// Clause: SysML 8.2.2.27 --
//
//   ExtendedDefinition : Definition =
//       BasicDefinitionPrefix? DefinitionExtensionKeyword+ 'def' Definition
//
// One `def`, then a Definition. This file writes `def` twice: `#X def def A;`, and then
// `#X def B;`. No corpus file writes `def` twice (`grep -rlE '\bdef\s+def\b'` over the
// pinned corpus finds none), so the corpus never shows which `def` the definition owns,
// nor where the second one goes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_extended_definition`
// (crates/sv2-syntax/src/parser/definition.rs) sees `#X def` and `extended_definition`
// takes the keyword and the first `def`. The second `def` is no identification, so
// `definition` builds an empty DefinitionDeclaration, and `definition_body` (same file)
// finds neither `;` nor `{` and reports the `def`, consuming nothing. The body loop hands
// `def A;` to `recover_statement` (crates/sv2-syntax/src/parser/recovery.rs), which wraps
// it in an Error beside the member, the `def` there no keyword but a name; its own
// diagnostic starts where the body's did, and `emit` keeps only the first:
//
//   PackageBody
//     PackageMember
//       ExtendedDefinition "#X def "    <- the space before the empty Definition is its own
//         DefinitionExtensionKeyword "#X"
//         KwDef "def"                   <- def_keyword(): the FIRST `def`
//         Definition ""                 <- definition(): present, zero width
//     Error "def A;"                    <- the second `def`, and A: not this definition's
//     PackageMember
//       ExtendedDefinition "#X def B;"  <- B is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding two ExtendedDefinitions.
//  2. The first owns exactly one `def`, the first, and its Definition spans nothing.
//     Something reported it: one diagnostic, on the second `def`.
//  3. The neighbours still answer. The first still owns its `#X`, and its Definition
//     declares nothing and has an empty body; B is whole: its keyword, its `def`, and a
//     Definition declaring B, closed by `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.27 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	#X def def A;
	#X def B;
}
