// Clause: SysML 8.2.2.25 --
//
//   UseCaseDefinition = OccurrenceDefinitionPrefix 'use' 'case' 'def'
//                       DefinitionDeclaration CaseBody
//
// The one definition in the family whose kind is two keywords. This file leaves out the
// second: `use def A;`, then `use case def B;`. No corpus file writes `use def` (`grep
// -rlE '\buse\s+def\b' vendor/corpus` finds none), so the corpus never shows what a `use`
// with no `case` after it becomes.
//
// Recovery (NOT a clause, and NOT an ADR): `at_case_definition` and `at_case_usage`
// (crates/sv2-syntax/src/parser/case.rs) both read a `CASES` row's keywords through
// `case_keywords_at`, which needs every one of them, in order, so neither begins at `use
// def`, and no other member does. Unlike `use case A def B;`, then, nothing is begun at
// all: `recover_statement` (crates/sv2-syntax/src/parser/recovery.rs) wraps the whole
// statement in an Error, the `use` there no keyword but a name, and reports it once, as
// unexpected, over the run it skipped:
//
//   PackageBody
//     Error "use def A;"                     <- no member, half-built or otherwise
//     PackageMember
//       UseCaseDefinition "use case def B;"  <- B is whole
//
// What that pins:
//
//  1. The parse still yields a tree, holding one UseCaseDefinition, B's.
//  2. `use def A;` is no UseCaseDefinition and no member of any kind: a `use` without its
//     `case` begins nothing, rather than a definition missing a keyword. Something
//     reported it: one diagnostic, over the Error.
//  3. The neighbours still answer. The package's only members are the package itself and
//     B, and B is whole, with its own three keywords, a declaration of B, and its `;`.
//
// ADR-0004 supports only the shape: every byte reaches the tree, so this round-trips.
// Do not cite 8.2.2.25 for the recovery outcome, and do not cite ADR-0022, which is about
// deviation diagnostics and says nothing about recovery.
package P {
	use def A;
	use case def B;
}
