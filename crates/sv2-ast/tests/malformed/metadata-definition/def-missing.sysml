// Clause: SysML 8.2.2.27 --
//
//   MetadataDefinition = ( isAbstract ?= 'abstract' )? DefinitionExtensionKeyword*
//                        'metadata' 'def' Definition
//   MetadataUsage      = UsageExtensionKeyword* ( '@' | 'metadata' )
//                        MetadataUsageDeclaration ( 'about' ... )? MetadataBody
//
// Both open with `metadata`; only the definition writes `def` after it. This file writes
// the `def` after the name, where neither production has it: `metadata A def B;`, then
// `metadata def C;`. No corpus file writes a `def` after a metadata name (`grep -rlE
// '\bmetadata\s+[A-Za-z_][A-Za-z0-9_]*\s+def\b'` over the pinned corpus finds none), so
// the corpus never shows what a `metadata` with no `def` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_metadata_definition`
// (crates/sv2-syntax/src/parser/metadata.rs) decides on `metadata` FOLLOWED BY `def` and
// on nothing less, so no MetadataDefinition is begun. `metadata A` is read as a
// MetadataUsage typed by A, whose body is missing and reported on the `def`;
// `recover_statement` (crates/sv2-syntax/src/parser/recovery.rs) wraps `def B;` in an
// Error beside it, and `emit` keeps only the first of the two diagnostics that start there:
//
//   PackageBody
//     PackageMember
//       MetadataUsage "metadata A "          <- not a MetadataDefinition
//     Error "def B;"
//     PackageMember
//       MetadataDefinition "metadata def C;" <- C is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one MetadataDefinition, C's.
//  2. A is no MetadataDefinition, half-built or otherwise: the absence is a missing node,
//     not a definition missing its `def`. Something reported it: one diagnostic, on the
//     `def`.
//  3. The neighbours still answer. The first member's element is a MetadataUsage, and C's
//     definition is whole: its own keywords and a Definition declaring C, closed by `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.27 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	metadata A def B;
	metadata def C;
}
