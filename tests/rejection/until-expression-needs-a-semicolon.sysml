// REJECTED BY RULE. The keyword until "introduces a Boolean-valued until expression
// terminated with a semicolon" (7.17.12, receipt b0446148).
// SysML 8.2.2.17.7 — WhileLoopNode = ... ( 'until' ExpressionParameterMember ';' )?
// rejects: PARSE-EXPECTED on: } until charge >= 100 }
action def Charge {
	loop {
		assign charge := MonitorBattery();
	} until charge >= 100 }
