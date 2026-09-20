// REJECTED BY RULE. A definition body reaches no ActionTargetSuccessionMember: an action
// usage in a part def is an OccurrenceUsageMember through DefinitionBodyItem, which has
// no target-succession suffix of either form.
// SysML 8.2.2.6.1   — DefinitionBodyItem = ... | NonOccurrenceUsageMember
//                                         | SourceSuccessionMember? OccurrenceUsageMember
// SysML 8.2.2.17.1  — ActionBodyItem = ... | SourceSuccessionMember? ActionBehaviorMember
//                                            ActionTargetSuccessionMember* | ...
part def P {
	action a;
	if x then b;
}
