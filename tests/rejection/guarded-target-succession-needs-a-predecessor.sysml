// REJECTED BY RULE. `if x then b;` is not an ActionBodyItem of its own: ActionBodyItem
// admits ActionTargetSuccessionMember only as a suffix of an InitialNodeMember or of an
// ActionBehaviorMember, and an action body that opens on the guard has neither. The
// fourth alternative does not rescue it: GuardedSuccession writes `first` or `succession`
// BEFORE its guard (8.2.2.17.8), and there is no `first` here.
// SysML 8.2.2.17.1  — ActionBodyItem = NonBehaviorBodyItem
//                                    | InitialNodeMember ActionTargetSuccessionMember*
//                                    | SourceSuccessionMember? ActionBehaviorMember
//                                      ActionTargetSuccessionMember*
//                                    | GuardedSuccessionMember
// SysML 8.2.2.17.8  — GuardedTargetSuccession = GuardExpressionMember
//                                               'then' TransitionSuccessionMember
// rejects: PARSE-UNEXPECTED on: if x then b;
action def A {
	if x then b;
}
