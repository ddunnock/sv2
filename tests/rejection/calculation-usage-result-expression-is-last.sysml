// REJECTED BY RULE. A calculation usage's body is a CalculationBody, whose part is
// `CalculationBodyItem* ResultExpressionMember?`: the result expression, if there is one,
// is the last thing in the body, and no item follows it. It is the expression's position
// that makes it the result. It is written parenthesised so that nothing but an expression
// can start there: a bare name would first be asked whether it opens a keywordless usage.
// SysML 8.2.2.19  — CalculationUsage = OccurrenceUsagePrefix 'calc'
//                                      ActionUsageDeclaration CalculationBody
// SysML 8.2.2.19  — CalculationBodyPart = CalculationBodyItem* ResultExpressionMember?
// rejects: PARSE-EXPECTED on: (a + b) attribute x;
part def P {
	calc c {
		(a + b) attribute x;
	}
}
