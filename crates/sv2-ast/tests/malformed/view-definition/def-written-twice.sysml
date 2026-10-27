// Clause: SysML 8.2.2.26.1 --
//
//   ViewDefinition = OccurrenceDefinitionPrefix 'view' 'def'
//                    DefinitionDeclaration ViewDefinitionBody
//
// One `def`, then the declaration and the body, which are the definition's own: there is
// no Definition node. This file writes `def` twice: `view def def A;`, and then `view def
// B;`. No corpus file writes `def` twice (`grep -rlE '\bdef\s+def\b' vendor/corpus` finds
// none), so the corpus never shows which `def` the definition owns, nor where the second
// one goes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_view_definition`
// (crates/sv2-syntax/src/parser/view.rs) sees `view def` and `view_definition` takes both
// keywords. The second `def` is no identification, so `definition_declaration` builds an
// empty declaration, and `view_definition_body` finds neither `;` nor `{` and reports the
// `def`, consuming nothing. The body loop hands `def A;` to `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs), which wraps it in an Error beside the
// member, the `def` there no keyword but a name; its own diagnostic starts where the
// body's did, and `emit` keeps only the first:
//
//   PackageBody
//     PackageMember
//       ViewDefinition "view def "      <- the space before the empty declaration is its own
//         OccurrenceDefinitionPrefix "" <- prefix(): present, empty
//         KwView "view"                 <- view_keyword()
//         KwDef "def"                   <- def_keyword(): the FIRST `def`
//         DefinitionDeclaration ""      <- declaration(): present, zero width
//         ViewDefinitionBody ""         <- body(): present, zero width
//     Error "def A;"                    <- the second `def`, and A: not this definition's
//     PackageMember
//       ViewDefinition "view def B;"    <- B is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding two ViewDefinitions.
//  2. The first owns exactly one `def`, the first, and its declaration and body span
//     nothing. Something reported it: one diagnostic, on the second `def`.
//  3. The neighbours still answer. The first's declaration declares nothing, and B is
//     whole: its own keywords, a declaration of B, and a body that is its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.26.1 for the recovery outcome, and do not cite ADR-0022, which is
// about deviation diagnostics and says nothing about recovery.
package P {
	view def def A;
	view def B;
}
