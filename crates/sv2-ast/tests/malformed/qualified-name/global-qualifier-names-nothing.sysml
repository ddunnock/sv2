// Clause: KerML 8.2.3.4.1 -- QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME
//
// The final NAME is NOT optional. Note 3 of that clause says a qualified name is "a
// sequence of segment names separated by `::` punctuation, optionally with the global
// scope qualifier `$` as an initial segment", and that the qualifier is an INITIAL
// segment -- it prefixes a name, it is not one. So `$::` on its own is a qualified name
// with no segments at all, which no valid text can write.
//
// The same text is pinned as a rejection case, tests/rejection/
// import-global-scope-without-name.sysml, which proves the parser REPORTS it. This
// fixture is about something the rejection corpus cannot say: what the ACCESSOR answers
// once it has been reported.
//
// Recovery (NOT a clause, and NOT an ADR): `qualified_name`
// (crates/sv2-syntax/src/parser/import.rs) takes the `$`, takes the `::` with
// `expect(ColonColon)`, then calls `expect_name("a name")`, which reports WITHOUT
// CONSUMING (`expect_name`, crates/sv2-syntax/src/parser/recovery.rs). So the node is
// built, it holds both punctuation tokens, and it holds no name token whatsoever:
//
//   QualifiedName "$::"
//     tok Dollar     "$"
//     tok ColonColon "::"
//                          <- and nothing else: no segment was written
//
// That is the shape worth pinning, because the three accessors must disagree with each
// other here in exactly the right way:
//
//   is_global()  -> true    the `$` really is there
//   segments()   -> empty   no segment name was written
//   name()       -> None    so there is no final segment to identify
//
// An accessor that answered `name()` from the node's text, or that treated the `$` or
// the `::` as a segment, would report a name the author never wrote. `is_global()` being
// true while `segments()` is empty is not a contradiction: the qualifier is punctuation
// about where resolution starts, not a name to resolve.
//
// ADR-0004 supports only the shape -- both tokens still reach the tree, so `$::`
// round-trips. It does not support this outcome, and neither does 8.2.3.4.1. Do not cite
// ADR-0022, which is about deviation diagnostics and says nothing about recovery.
public import $::;
