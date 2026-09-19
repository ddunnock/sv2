// REJECTED BY RULE. The member a SourceSuccessionMember prefixes is an occurrence usage's:
// OccurrenceUsageMember in a definition body, StructureUsageMember or ActionBehaviorMember
// in an action body. An attribute is a NonOccurrenceUsageElement, and no item production
// puts `then` before a NonOccurrenceUsageMember.
// SysML 8.2.2.6.1   — DefinitionBodyItem = ... | NonOccurrenceUsageMember
//                                         | SourceSuccessionMember? OccurrenceUsageMember
// SysML 8.2.2.6.4   — NonOccurrenceUsageElement = ... | AttributeUsage | ...
part def P {
	then attribute x;
}
