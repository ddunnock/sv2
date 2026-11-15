// Clause: SysML 8.2.2.27 --
//
//   MetadataDefinition = ( isAbstract ?= 'abstract' )? DefinitionExtensionKeyword*
//                        'metadata' 'def' Definition
//
// and SysML 8.2.2.6.1 --
//
//   BasicDefinitionPrefix = isAbstract ?= 'abstract' | isVariation ?= 'variation'
//
// A metadata definition's prefix is its own, and narrower than DefinitionPrefix: it may
// write `abstract`, never `variation`. This file writes `variation` on one:
// `variation metadata def V;`. No corpus file writes it (`grep -rlE
// '\bvariation\s+metadata\b'` over the pinned corpus finds none), so the corpus never
// shows what becomes of the keyword the production leaves out.
//
// Recovery (NOT a clause, and NOT an ADR): `at_metadata_definition`
// (crates/sv2-syntax/src/parser/metadata.rs) skips an `abstract` and `#` keywords only,
// so it does not see a definition at `variation`, and no member begins there.
// `recover_statement` (crates/sv2-syntax/src/parser/recovery.rs) wraps `variation` alone
// in an Error and reports it as unexpected; the next member then begins at `metadata def`:
//
//   PackageBody
//     Error "variation"                      <- not this definition's, nor any prefix's
//     PackageMember
//       MetadataDefinition "metadata def V;" <- V is whole, and not abstract
//
// What that pins:
//
//  1. The parse still yields a tree, holding one MetadataDefinition, V's.
//  2. V owns no `variation`, and has no `abstract` to answer in its place: the keyword is
//     outside the node, in an Error. Something reported it: one diagnostic, over the
//     Error.
//  3. The neighbours still answer. V is whole: its own keywords, and a Definition
//     declaring V, closed by `;`; and the package's one member is V's.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.27 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	variation metadata def V;
}
