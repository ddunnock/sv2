// REJECTED BY RULE. A definition body has no ActionNodeMember: DefinitionBodyItem owns
// usages through NonOccurrenceUsageMember and OccurrenceUsageMember, and a MergeNode is
// neither kind of UsageElement -- it is an ActionNode, reached only from ActionBodyItem.
// The `then` before it does not help: DefinitionBodyItem's SourceSuccessionMember
// prefixes an OccurrenceUsageMember.
// SysML 8.2.2.6.1   — DefinitionBodyItem = ... | NonOccurrenceUsageMember
//                                         | SourceSuccessionMember? OccurrenceUsageMember
// SysML 8.2.2.17.1  — ActionBehaviorMember = BehaviorUsageMember | ActionNodeMember
// SysML 8.3.17.6    — validateControlNodeOwningType: the owningType is an
//                     ActionDefinition or ActionUsage (receipt 695df335)
// rejects: PARSE-UNEXPECTED on: merge m;
part def P {
	merge m;
}
