// Clause: SysML 8.2.2.26.3 --
//
//   ViewpointDefinition = OccurrenceDefinitionPrefix 'viewpoint' 'def'
//                         DefinitionDeclaration RequirementBody
//
// One `def`, then the declaration and the body, which are the definition's own: there is
// no Definition node. This file writes `def` twice: `viewpoint def def A;`, and then
// `viewpoint def B;`. No corpus file writes `def` twice (`grep -rlE '\bdef\s+def\b'
// vendor/corpus` finds none), so the corpus never shows which `def` the definition owns,
// nor where the second one goes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_viewpoint_definition`
// (crates/sv2-syntax/src/parser/view.rs) sees `viewpoint def` and `viewpoint_definition`
// takes both keywords. The second `def` is no identification, so `definition_declaration`
// builds an empty declaration, and `requirement_body` finds neither `;` nor `{` and
// reports the `def`, consuming nothing. The body loop hands `def A;` to
// `recover_statement` (crates/sv2-syntax/src/parser/recovery.rs), which wraps it in an
// Error beside the member, the `def` there no keyword but a name; its own diagnostic
// starts where the body's did, and `emit` keeps only the first:
//
//   PackageBody
//     PackageMember
//       ViewpointDefinition "viewpoint def "   <- the space before the empty declaration is its own
//         OccurrenceDefinitionPrefix ""        <- prefix(): present, empty
//         KwViewpoint "viewpoint"              <- viewpoint_keyword()
//         KwDef "def"                          <- def_keyword(): the FIRST `def`
//         DefinitionDeclaration ""             <- declaration(): present, zero width
//         RequirementBody ""                   <- body(): present, zero width
//     Error "def A;"                           <- the second `def`, and A: not this definition's
//     PackageMember
//       ViewpointDefinition "viewpoint def B;" <- B is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding two ViewpointDefinitions.
//  2. The first owns exactly one `def`, the first, and its declaration and body span
//     nothing. Something reported it: one diagnostic, on the second `def`.
//  3. The neighbours still answer. The first's declaration declares nothing, and B is
//     whole: its own keywords, a declaration of B, and a body that is its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.26.3 for the recovery outcome, and do not cite ADR-0022, which is
// about deviation diagnostics and says nothing about recovery.
package P {
	viewpoint def def A;
	viewpoint def B;
}
