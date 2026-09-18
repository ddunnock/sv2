// REJECTED BY RULE. A PositionalArgumentList is `ArgumentMember ( ',' ArgumentMember )*`
// and states no trailing comma.
// KerML 8.2.5.8.3 — PositionalArgumentList = ArgumentMember ( ',' ArgumentMember )*
// This is worth holding because the NEIGHBOURING list production does admit one:
// KerML 8.2.5.8.2 — SequenceExpressionList = OwnedExpression ( ',' SequenceExpressionList
//                                                            | ',' )?
// so `( a , )` is a well-formed sequence and `f( a , )` is not a well-formed argument
// list. The two are three clauses apart and share the comma, which is exactly how the
// trailing comma gets copied from one to the other.
calc def C {
	f(a,)
}
