// REJECTED BY RULE. `first` is an ActionBodyItem, and a definition body is not an action
// body: DefinitionBodyItem has no ActionBodyItem alternative.
// SysML 8.2.2.6.1  — DefinitionBodyItem = DefinitionMember | VariantUsageMember
//                                       | NonOccurrenceUsageMember
//                                       | SourceSuccessionMember? OccurrenceUsageMember
//                                       | AliasMember | Import
// SysML 8.2.2.17.1 — ActionBodyItem     = NonBehaviorBodyItem
//                                       | InitialNodeMember ActionTargetSuccessionMember*
//                                       | …
part def V {
	first start;
}
