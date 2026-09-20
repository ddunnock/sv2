// REJECTED BY RULE. A definition body reaches no ActionTargetSuccessionMember at all, so
// none of the three successions is an item there: an action usage in a part def is an
// OccurrenceUsageMember through DefinitionBodyItem, which has no such suffix.
// SysML 8.2.2.6.1   — DefinitionBodyItem = ... | NonOccurrenceUsageMember
//                                         | SourceSuccessionMember? OccurrenceUsageMember
// SysML 8.2.2.17.1  — ActionBodyItem = ... | SourceSuccessionMember? ActionBehaviorMember
//                                            ActionTargetSuccessionMember* | ...
part def P {
	action a;
	else b;
}
