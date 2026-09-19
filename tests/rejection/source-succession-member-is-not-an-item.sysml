// REJECTED BY RULE. SourceSuccessionMember is a prefix, never an item alone: every item
// production that has it pairs it with the occurrence usage member after it.
// SysML 8.2.2.9.3   — SourceSuccessionMember = 'then' SourceSuccession
// SysML 8.2.2.17.1  — ActionBodyItem = ... | SourceSuccessionMember? ActionBehaviorMember
//                                            ActionTargetSuccessionMember* | ...
//                     NonBehaviorBodyItem = ... | SourceSuccessionMember? StructureUsageMember
// A bare `then;` is not a TargetSuccession either: it has no ConnectorEnd.
action def A {
	then;
}
