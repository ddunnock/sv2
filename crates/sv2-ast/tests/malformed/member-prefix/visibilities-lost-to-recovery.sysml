// Clause: SysML 8.2.2.5.1 -- `MemberPrefix : Membership = ( visibility =
// VisibilityIndicator )?`
//
// Every part of that production is optional, so NOTHING can make a MemberPrefix's own
// shape wrong: in any tree, in any state of damage, it holds either nothing or one
// VisibilityIndicator. tests/sweep.rs asserts exactly that over all 610 trees. This
// fixture therefore does not pin a violated shape, because there is none to pin. It pins
// the one thing that CAN go wrong for this accessor, which is ATTRIBUTION.
//
// The clause says a visibility written in front of a member belongs to that member. It
// says nothing about a visibility written in front of something that turns out not to be
// a member at all.
//
// Recovery (NOT a clause, and NOT an ADR): `private protected` is not a member --
// `at_visibility` holds, but the token after the first word is not one that begins a
// member element, so no membership production is entered. `recover_statement`
// (crates/sv2-syntax/src/parser/recovery.rs) opens one SyntaxKind::Error, takes the first
// token always, and runs on until a member keyword; `part` is one, so the Error closes
// holding BOTH words and `part def A;` survives as a member of its own:
//
//   RootNamespace
//     Error "private protected"      <- both words, one node
//     PackageMember "part def A;"
//       MemberPrefix ""              <- reports NEITHER
//       PartDefinition "part def A;"
//
// Two properties of that make it the case worth writing, and `private private` would
// show neither:
//
//  1. TWO visibilities are lost, and they DISAGREE. An accessor that went looking at a
//     sibling or at preceding text to find "the visibility the author meant" has two
//     candidates and no principled way to choose. There is no answer to invent, which is
//     the clearest possible statement that it must not invent one.
//  2. The parser raises ONE diagnostic for the pair, so the number of diagnostics is not
//     a measure of how much was lost. A caller counting them to decide how much to
//     distrust the tree would under-count here.
//
// ADR-0004 supports only the shape -- every token still enters the tree, so the parse
// stays lossless. Do not cite 8.2.2.5.1 for the attribution, and do not cite ADR-0022,
// which is about deviation diagnostics and strict conformance and says nothing about
// recovery.
private protected part def A;
