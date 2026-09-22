// REJECTED BY RULE. A constructor expression writes its argument list, even when it is
// empty: ConstructorResult = ArgumentList, and ArgumentList's parentheses are not optional.
// "the keyword new followed by the qualified name of a type to be instantiated ...
// followed by a parenthesized list of argument expressions" (KerML 7.4.9.4, receipt
// f77ceb64).
// KerML 8.2.5.8.3 — ConstructorExpression = 'new' InstantiatedTypeMember ConstructorResultMember
//                   ConstructorResult     = ArgumentList
// rejects: PARSE-EXPECTED on: new A }
calc def C {
	new A }
