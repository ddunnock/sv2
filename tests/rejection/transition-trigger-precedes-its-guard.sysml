// REJECTED BY RULE. In a transition the accepter comes before the guard. 7.18.3's OnOff4
// example (receipt 6e6e9493) writes `if isEnabled accept TurnOn …`, against its own prose
// and its OnOff3; the grammar is followed.
// SysML 8.2.2.18.3 — TransitionUsage = 'transition' ( UsageDeclaration 'first' )?
//                      FeatureChainMember EmptyParameterMember
//                      ( EmptyParameterMember TriggerActionMember )?
//                      GuardExpressionMember? EffectBehaviorMember? 'then' …
// rejects: PARSE-EXPECTED on: transition t first a if g accept S then b;
state def D {
	state a;
	transition t first a if g accept S then b;
}
