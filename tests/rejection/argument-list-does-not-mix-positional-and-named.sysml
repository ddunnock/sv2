// REJECTED BY RULE. PositionalArgumentList and NamedArgumentList are ALTERNATIVES of
// ArgumentList, so a list may be one or the other and never both.
// KerML 8.2.5.8.3 — ArgumentList           = '(' ( PositionalArgumentList
//                                                | NamedArgumentList )? ')'
//                   PositionalArgumentList = ArgumentMember ( ',' ArgumentMember )*
//                   NamedArgumentList      = NamedArgumentMember
//                                            ( ',' NamedArgumentMember )*
// The second argument is written as a NamedArgument, and this list has already
// committed to the positional alternative at the first.
// `q = 1` is also not readable as a positional argument, because an ArgumentMember's
// value is an OwnedExpression (8.2.5.8.1) and `=` is not an infix operator in table 6
// of 8.2.5.8.1 — so neither alternative admits this line.
calc def C {
	f(a, q = 1)
}
