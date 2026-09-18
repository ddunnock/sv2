// REJECTED BY RULE. The sibling of
// tests/rejection/argument-list-does-not-mix-positional-and-named.sysml, taken from the
// other side: this list commits to the NAMED alternative at the first argument and then
// writes a positional one.
// KerML 8.2.5.8.3 — ArgumentList      = '(' ( PositionalArgumentList
//                                           | NamedArgumentList )? ')'
//                   NamedArgumentList = NamedArgumentMember ( ',' NamedArgumentMember )*
//                   NamedArgument     = ParameterRedefinition '=' ArgumentValue
// Both directions are held, because a parser that chose its alternative from the FIRST
// argument and then read the rest loosely would accept one of them and not the other.
calc def C {
	f(q = 1, a)
}
