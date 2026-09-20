// REJECTED BY RULE. Only ActionBehaviorMember takes the ActionTargetSuccessionMember*
// suffix. A part in an action body is a StructureUsageMember, read through
// NonBehaviorBodyItem, which has no such suffix in any of the three forms.
// SysML 8.2.2.17.1  — ActionBodyItem = ... | SourceSuccessionMember? ActionBehaviorMember
//                                            ActionTargetSuccessionMember* | ...
//                     NonBehaviorBodyItem = ... | SourceSuccessionMember? StructureUsageMember
// rejects: PARSE-UNEXPECTED on: else b;
action def A {
	part p;
	else b;
}
