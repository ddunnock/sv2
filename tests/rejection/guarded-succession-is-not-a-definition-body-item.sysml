// REJECTED BY RULE. GuardedSuccessionMember is an ActionBodyItem alternative, and a
// definition body reaches only DefinitionBodyItem, which has no such alternative. The
// same holds of a requirement body, which reaches DefinitionBodyItem too (8.2.2.21.1).
// SysML 8.2.2.6.1   — DefinitionBodyItem = Import | AliasMember | DefinitionMember
//                                        | VariantUsageMember | NonOccurrenceUsageMember
//                                        | SourceSuccessionMember? OccurrenceUsageMember
// SysML 8.2.2.17.1  — ActionBodyItem = ... | GuardedSuccessionMember
// rejects: PARSE-UNEXPECTED on: first a if x then b;
part def P {
	first a if x then b;
}
