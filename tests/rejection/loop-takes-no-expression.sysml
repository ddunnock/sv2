// REJECTED BY RULE. "The keyword loop may be used as a shorthand for while true"
// (7.17.12, receipt b0446148): `loop` owns an EmptyParameterMember in the while slot,
// and no expression follows it.
// SysML 8.2.2.17.7 — WhileLoopNode = ActionNodePrefix
//                    ( 'while' ExpressionParameterMember | 'loop' EmptyParameterMember )
//                    ActionBodyParameterMember ( 'until' ExpressionParameterMember ';' )?
// rejects: PARSE-EXPECTED on: loop charge < 100 {
action def Charge {
	loop charge < 100 {
		assign charge := MonitorBattery();
	}
}
