// REJECTED BY RULE. `else b;` is not an ActionBodyItem of its own: ActionBodyItem admits
// ActionTargetSuccessionMember only as a suffix of an InitialNodeMember or of an
// ActionBehaviorMember, whichever of the three successions it writes, and an action body
// that opens on `else` has neither.
// SysML 8.2.2.17.1  — ActionBodyItem = NonBehaviorBodyItem
//                                    | InitialNodeMember ActionTargetSuccessionMember*
//                                    | SourceSuccessionMember? ActionBehaviorMember
//                                      ActionTargetSuccessionMember*
//                                    | GuardedSuccessionMember
// SysML 8.2.2.17.8  — DefaultTargetSuccession = 'else' TransitionSuccessionMember
action def A {
	else b;
}
