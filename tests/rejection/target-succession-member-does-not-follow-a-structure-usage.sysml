// REJECTED BY RULE. Only ActionBehaviorMember takes the ActionTargetSuccessionMember*
// suffix. A part in an action body is a StructureUsageMember, read through
// NonBehaviorBodyItem, which has no such suffix; so `then b;` after it is no item.
// SysML 8.2.2.17.1  — ActionBodyItem = NonBehaviorBodyItem
//                                    | InitialNodeMember ActionTargetSuccessionMember*
//                                    | SourceSuccessionMember? ActionBehaviorMember
//                                      ActionTargetSuccessionMember*
//                                    | GuardedSuccessionMember
//                     NonBehaviorBodyItem = ... | SourceSuccessionMember? StructureUsageMember
action def A {
	part p;
	then b;
}
