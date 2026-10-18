// Clause: SysML 8.2.2.5.1 -- `'alias' ( '<' memberShortName = NAME '>' )?
// ( memberName = NAME )?`. The short-name slot holds a NAME. The text below opens and
// closes it with nothing in it, which the clause does not admit, and says nothing about.
//
// Recovery (NOT a clause, and NOT an ADR): `alias_member` reads `<` and then calls
// `expect_name("a short name")` (crates/sv2-syntax/src/parser/namespace.rs), which
// REPORTS without consuming, so the `>` is still next and closes the slot immediately.
// `Car` therefore arrives with the slot already shut.
//
// This is the OTHER branch of the rule the unclosed case pins, and it is here because the
// first formulation of that rule got this wrong. "The first NAME after `<` is the short
// name" is correct for `alias <c Car for X;` and WRONG here: it would report `Car` as the
// short name and leave the member name empty, inverting both slots. Only the full rule --
// `<` opens the slot, `>` CLOSES it, and the slot holds at most one name -- is right in
// both. Two fixtures, because one of them passes either formulation.
//
// ADR-0004 supports only the shape: every token still enters the tree. Do not cite
// 8.2.2.5.1 for the empty slot, and do not cite ADR-0022.
alias <> Car for Automobile;
