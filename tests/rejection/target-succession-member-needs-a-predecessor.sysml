// REJECTED BY RULE. `then X;` is not an ActionBodyItem of its own: ActionBodyItem admits
// ActionTargetSuccessionMember only as a suffix of an InitialNodeMember or of an
// ActionBehaviorMember, and an action body that opens on `then NAME;` has neither.
// SysML 8.2.2.17.1  — ActionBodyItem = NonBehaviorBodyItem
//                                    | InitialNodeMember ActionTargetSuccessionMember*
//                                    | SourceSuccessionMember? ActionBehaviorMember
//                                      ActionTargetSuccessionMember*
//                                    | GuardedSuccessionMember
// SysML 8.2.2.6.4   — StructureUsageElement opens on a keyword in every alternative, so
//                     NonBehaviorBodyItem's `SourceSuccessionMember? StructureUsageMember`
//                     cannot read `then a;` either.
action def A {
	then a;
}
