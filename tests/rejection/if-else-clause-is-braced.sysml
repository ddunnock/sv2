// REJECTED BY RULE. The else clause is braced as the then clause is (7.17.11, receipt
// 5a98cecc): an ActionBodyParameterMember, or an IfNode, and no `;` body.
// SysML 8.2.2.17.7 — IfNode = … ( 'else' ( ActionBodyParameterMember
//                                        | IfNodeParameterMember ) )?
// rejects: PARSE-EXPECTED on: } else action main : MaintainSpeed;
action def Cruise {
	if speed < lowerLimit {
		action increase : IncreaseSpeed;
	} else action main : MaintainSpeed;
}
