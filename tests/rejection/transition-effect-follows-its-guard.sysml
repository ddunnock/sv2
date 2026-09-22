// REJECTED BY RULE. A transition writes its accepter, its guard and its effect in that
// order; "it is also placed between the source and target parts, after the guard and
// accepter" (7.18.3, receipt 6e6e9493).
// SysML 8.2.2.18.3 — TransitionUsage = … ( EmptyParameterMember TriggerActionMember )?
//                      GuardExpressionMember? EffectBehaviorMember? 'then' …
// rejects: PARSE-EXPECTED on: transition t first a do action x if g then b;
state def D {
	state a;
	transition t first a do action x if g then b;
}
