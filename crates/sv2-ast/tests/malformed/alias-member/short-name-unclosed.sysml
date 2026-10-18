// Clause: SysML 8.2.2.5.1 (the AliasMember production, as
// crates/sv2-syntax/src/parser/namespace.rs states it at `alias_member`) --
//
//   AliasMember : Membership =
//       MemberPrefix
//       'alias' ( '<' memberShortName = NAME '>' )?
//       ( memberName = NAME )?
//       'for' memberElement = [QualifiedName]
//       RelationshipBody
//
// Two name slots, both optional, both 0..1 on the metaclass, and both spelled NAME. The
// only thing that tells them apart in the text is POSITION: the short name is the one
// inside the angle brackets. `Membership`, not `OwningMembership` — KerML 8.3.2.4.3 — so
// the target is a reference to an element declared elsewhere, not a nested element.
//
// The specification says the short name is written `'<' NAME '>'`. It says nothing about
// what the two slots mean when the closing `>` is not written.
//
// Recovery (NOT a clause, and NOT an ADR): `alias_member` reads `<`, takes a name, and
// then calls `self.expect(SyntaxKind::Gt, "`>`")`, which REPORTS a diagnostic rather than
// consuming anything. It then goes on to read the optional member name as usual. So in
// the text below both names are present, as adjacent sibling tokens of the same kind,
// with no `>` between them:
//
//   AliasMember
//     MemberPrefix ""
//     KwAlias "alias"
//     Lt "<"
//     BasicName "c"        <- memberShortName
//     BasicName "Car"      <- memberName
//     KwFor "for"
//     QualifiedName "Automobile"
//     RelationshipBody ";"
//
// This is the case that matters for a typed accessor, and it is why this condition was
// chosen over `<>` or a missing `for`. Both names are BasicName tokens, so nothing about
// the KIND distinguishes them, and the delimiter a positional rule would naturally key on
// is the one thing missing. An accessor that defines the short name as "the name BETWEEN
// `<` and `>`" reports none here and then hands `c` back as the member name — confidently
// wrong about both slots at once, from a single missing character. The rule that survives
// is: `<` opens the slot, `>` closes it, and the slot holds at most ONE name, so a second
// name inside an unclosed slot falls through to the member name.
//
// The `<>` case is the other half of that rule and is well formed enough to live with the
// positive cases: there the slot is opened and closed with nothing in it, so the name
// after it is the member name and the short name is absent.
//
// ADR-0025 sanctions pinning this: where the grammar admits one shape and the tree
// carries another, the case pins which. ADR-0004 supports only the shape — every token
// still enters the tree, so the parse stays lossless — never this outcome. Do not cite
// 8.2.2.5.1 for the fall-through, and do not cite ADR-0022, which is about deviation
// diagnostics and strict conformance and says nothing about recovery.
alias <c Car for Automobile;
