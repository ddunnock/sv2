// REJECTED BY RULE. The condition is "followed by a then clause" (7.17.11, receipt
// 5a98cecc); only the else clause is optional.
// SysML 8.2.2.17.7 — IfNode = ActionNodePrefix 'if' ExpressionParameterMember
//                             ActionBodyParameterMember ( 'else' … )?
// rejects: PARSE-EXPECTED on: if selectedSensor == null else {
action def Read {
	if selectedSensor == null else {
		assign reading := undefinedValue;
	}
}
