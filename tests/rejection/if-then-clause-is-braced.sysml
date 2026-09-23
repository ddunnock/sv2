// REJECTED BY RULE. "Each of the then-clause and the else-clause is itself notated as an
// action usage, but with the body required to be given using curly braces { … }, with a
// semicolon not allowed for an empty body" (7.17.11, receipt 5a98cecc).
// SysML 8.2.2.17.7 — IfNode = ActionNodePrefix 'if' ExpressionParameterMember
//                             ActionBodyParameterMember ( 'else' … )?
//                    ActionBodyParameter = ( 'action' UsageDeclaration? )?
//                                          '{' ActionBodyItem* '}'
// rejects: PARSE-EXPECTED on: if speed < lowerLimit action increase : IncreaseSpeed;
action def Cruise {
	if speed < lowerLimit action increase : IncreaseSpeed;
}
