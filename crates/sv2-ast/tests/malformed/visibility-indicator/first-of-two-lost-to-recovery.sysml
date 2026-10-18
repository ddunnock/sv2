// Clause: KerML 8.2.3.4.1 and SysML 8.2.2.5.1 --
//
//   VisibilityIndicator = 'public' | 'private' | 'protected'
//
// Three keywords and nothing else, so a VisibilityIndicator's own shape CANNOT be wrong:
// every one of the 821 in this repository holds exactly one keyword token and no child
// node, which tests/sweep.rs asserts. Nor is an EMPTY one reachable. `visibility_indicator`
// (crates/sv2-syntax/src/parser/import.rs) does have an error path -- it reports
// "`public`, `private` or `protected`" without consuming -- but neither caller can take
// it: `body_elements` enters only on `at_visibility`, and a bare `import A::*;` never
// reaches `import()` at all (it is reported as "unexpected `import`"). The path is there
// so that a FUTURE caller cannot lose a token, not because anything exercises it.
//
// So this fixture pins the one thing that can go wrong, which is ATTRIBUTION -- and the
// answer here is the opposite of the obvious one.
//
// Recovery (NOT a clause, and NOT an ADR): `public private import A::*;` is not an import.
// `at_import` wants a visibility followed by `import`, and after `public` comes `private`,
// so the import is not recognised at `public`. `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) takes the first token always and runs to the
// next member keyword, so `public` alone lands in an Error; `private import A::*;` is then
// a clean import of its own:
//
//   RootNamespace
//     Error "public"                  <- reported, and belongs to nobody
//     Import "private import A::*;"
//       VisibilityIndicator "private" <- the import's own, and it is the SECOND word
//
// That makes it the case worth writing rather than `public public`:
//
//  1. A visibility IS reported, so this is not the empty-prefix situation that
//     tests/malformed/member-prefix/visibilities-lost-to-recovery.sysml already pins.
//     The accessor must report `private` -- not None, and not `public`.
//  2. The reported one is the LATER word. Anything that reasoned "the visibility written
//     in front of this import" by looking leftwards past the node, or that took the first
//     visibility in the statement, would answer `public`. The two words disagree, so
//     there is no spelling of that mistake that happens to be right.
//  3. `public` really is in the tree, in its own Error node, so the parse stays lossless
//     (ADR-0004) and the test can prove the word was not simply dropped.
//
// Do not cite 8.2.3.4.1 for the recovery outcome, and do not cite ADR-0022, which is
// about deviation diagnostics and says nothing about recovery.
public private import A::*;
