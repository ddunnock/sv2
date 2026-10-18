// Clause: SysML 8.2.2.5.1 and KerML 8.2.5.13 --
//
//   ElementFilterMember : MemberPrefix 'filter' OwnedExpression ';'
//
// The OwnedExpression is NOT optional. Every other slot in this production either is
// optional (MemberPrefix, which is `( visibility = VisibilityIndicator )?`) or is a
// literal the parser only reaches by having matched it (`filter`, `;`), so the condition
// is the one part of this production a well-formed file cannot leave out -- and the only
// one whose absence the accessor has to answer for.
//
// Nothing in this repository's 610 trees writes a filter without a condition: 25
// ElementFilterMember nodes across the corpus and rejection sets, every one of them
// carrying exactly one. So this file is hand written for precisely the reason
// .claude/rules/tests.md allows one -- a distinction the pinned corpus does not contain
// -- and its expectation still comes from the production above, never from what the
// accessor printed.
//
// Recovery (NOT a clause, and NOT an ADR): `owned_expression` climbs to
// `base_expression`, whose final `else` is `error_expected("an expression")`
// (crates/sv2-syntax/src/parser/primary.rs). `error_expected` REPORTS WITHOUT CONSUMING,
// so no node is built for the slot and the `;` is still there for the member's own
// `expect` to take:
//
//   ElementFilterMember "private filter ;"
//     MemberPrefix "private"           <- survives, and still reports `private`
//       VisibilityIndicator "private"
//     tok KwFilter "filter"
//     tok Semicolon ";"                <- the member closed normally
//                                      <- and NOTHING stands where the condition goes
//
// Two things make that the shape worth pinning:
//
//  1. The slot is EMPTY, not an Error node. `filter()` must therefore report None, and
//     a caller cannot distinguish "absent" from "damaged" by finding a
//     SyntaxKind::Error there, because there is none to find. The diagnostic is the only
//     record that something was written here and could not be read, which is why the
//     test reads Parse::errors() as well as the tree: absence that nothing reported is
//     absence guessed (ADR-0025).
//  2. The member still closed. The damage is confined to one slot, so the prefix beside
//     it keeps its visibility -- the neighbour has something real to answer, and the
//     answer is `private`, not None. An accessor that treated a damaged member as
//     wholly untrustworthy would lose a fact that is still there. The import above the
//     filter is here for the same reason: it is a second neighbour, in the same body,
//     with a visibility of its own, and it must come back whole.
//
// ADR-0004 supports only the shape: every byte still enters the tree, so the parse stays
// lossless and `private filter ;` round-trips. It does not support this outcome, and
// neither does 8.2.2.5.1. Do not cite ADR-0022, which is about deviation diagnostics and
// strict conformance and says nothing about recovery.
package P {
	public import V::**;
	private filter ;
}
