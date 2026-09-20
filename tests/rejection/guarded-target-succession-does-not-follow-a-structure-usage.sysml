// REJECTED BY RULE. Only ActionBehaviorMember takes the ActionTargetSuccessionMember*
// suffix, in the guarded form as in the plain one. A part in an action body is a
// StructureUsageMember, read through NonBehaviorBodyItem, which has no such suffix.
// SysML 8.2.2.17.1  — ActionBodyItem = ... | SourceSuccessionMember? ActionBehaviorMember
//                                            ActionTargetSuccessionMember* | ...
//                     NonBehaviorBodyItem = ... | SourceSuccessionMember? StructureUsageMember
// rejects: PARSE-UNEXPECTED on: if x then b;
action def A {
	part p;
	if x then b;
}
