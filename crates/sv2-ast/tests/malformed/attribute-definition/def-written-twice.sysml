// Clause: SysML 8.2.2.7 --
//
//   AttributeDefinition = DefinitionPrefix 'attribute' 'def' Definition
//
// One `def`, then the Definition. This file writes `def` twice: `attribute def def A;`,
// and then `attribute def B;`. No corpus file writes `def` twice (`grep -rlE
// '\bdef\s+def\b' vendor/corpus` finds none), so the corpus never shows which `def` the
// definition owns, nor where the second one goes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_simple_definition`
// (crates/sv2-syntax/src/parser/definition.rs) sees `attribute def` and
// `simple_definition` takes both keywords. The second `def` is no identification, so
// `definition` builds an empty declaration, and `definition_body` finds neither `;` nor
// `{` and reports the `def` with `error_expected`, consuming nothing. The body loop then
// hands `def A;` to `recover_statement` (crates/sv2-syntax/src/parser/recovery.rs), which
// wraps it in an Error beside the member, the `def` there no keyword but a name; its own
// diagnostic starts where the body's did, and `emit` keeps only the first:
//
//   PackageBody
//     PackageMember
//       AttributeDefinition "attribute def "   <- the space before the empty Definition is its own
//         DefinitionPrefix ""                  <- prefix(): present, empty
//         KwAttribute "attribute"              <- attribute_keyword()
//         KwDef "def"                          <- def_keyword(): the FIRST `def`
//         Definition ""                        <- definition(): present, zero width
//     Error "def A;"                           <- the second `def`, and A: not this definition's
//     PackageMember
//       AttributeDefinition "attribute def B;" <- B is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding two AttributeDefinitions.
//  2. The first owns exactly one `def`, the first, and its definition spans nothing: no
//     name and no body. Something reported it: one diagnostic, on the second `def`.
//  3. The neighbours still answer. The Definition of the first declares nothing, and B
//     is whole: its own keywords, and a Definition declaring B with its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.7 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	attribute def def A;
	attribute def B;
}
