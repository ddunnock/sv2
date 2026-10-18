// Clause: SysML 8.2.2.5.1 (.claude/state/grammar/units/PackageMember@sysml.json,
// spec_clause_ref region_sha256 3ee603b1b89d4345) --
//
//   PackageMember : OwningMembership =
//       MemberPrefix ( ownedRelatedElement += DefinitionElement
//                    | ownedRelatedElement = UsageElement )
//   MemberPrefix : Membership = ( visibility = VisibilityIndicator )?
//
// So a visibility belongs to the prefix of the member it introduces, and a prefix with
// no element after it is not a member at all. That is all the specification establishes:
// it says what a member is, and says nothing about where a visibility goes when the
// element it was meant to introduce is not there. (PackageMember is SysML-scoped only,
// ADR-0015, so this case is .sysml and there is no KerML counterpart to write.)
//
// Recovery (NOT a clause, and NOT an ADR): `private ;` matches no body alternative, so
// `recover_statement` in crates/sv2-syntax/src/parser/recovery.rs opens a
// SyntaxKind::Error, always takes the first token, and stops at a `;` at the depth it
// started from. Both tokens therefore land in an Error node that is a SIBLING of the
// following member. The `part def A;` after it survives as its own PackageMember because
// `at_member_keyword` ends the recovery run — the same function's doc states that, so
// the valid item after a missing `;` is read as an item rather than swallowed.
//
// Two consequences a typed accessor must not get wrong, and this is why the case is here:
//
//  1. ATTRIBUTION. The surviving member's MemberPrefix is present and EMPTY. The author
//     wrote `private` immediately before it, and it is not this member's visibility. An
//     accessor that looked at preceding text, or at a sibling, would report a visibility
//     the member does not have. This is the failure neither a presence check nor an
//     absence check sees: nothing is missing from the tree, and nothing is empty that
//     should be full — the text is simply attached somewhere else.
//  2. The orphaned `private` is lexed as a BasicName in that position, not a KwPrivate,
//     because it is not a keyword there. So even a consumer that went looking for a
//     visibility token in the Error node would not find one of the right kind.
//
// ADR-0025 sanctions pinning this: where the grammar admits one shape and the tree
// carries another, the case pins which. ADR-0004 supports only the shape — every token
// still enters the tree, so the parse stays lossless — never this outcome. Do not cite
// 8.2.2.5.1 for the attribution, and do not cite ADR-0022, which is about deviation
// diagnostics and strict conformance and says nothing about recovery.
package P { private ; part def A; }
