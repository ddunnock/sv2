// REJECTED BY RULE. A for loop's variable declaration is "followed by the keyword in and
// a sequence expression" (7.17.12, receipt b0446148).
// SysML 8.2.2.17.7 — ForLoopNode = ActionNodePrefix 'for' ForVariableDeclarationMember
//                                  'in' NodeParameterMember ActionBodyParameterMember
// rejects: PARSE-EXPECTED on: for vehiclePower powerProfile {
action def Drive {
	for vehiclePower powerProfile {
		assign position := ComputeDynamics(position, vehiclePower);
	}
}
