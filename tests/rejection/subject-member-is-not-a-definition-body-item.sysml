// REJECTED BY RULE. A SubjectMember is an alternative of RequirementBodyItem and of
// CaseBodyItem, and of no other body. A definition body does not reach one.
// SysML 8.2.2.6.1  — DefinitionBodyItem  = DefinitionMember | VariantUsageMember
//                                        | NonOccurrenceUsageMember
//                                        | SourceSuccessionMember? OccurrenceUsageMember
//                                        | AliasMember | Import
// SysML 8.2.2.21.1 — RequirementBodyItem = DefinitionBodyItem | SubjectMember | ...
// The same eight characters are a member in a requirement body and not a construct at
// all here. SysML 8.3.21.11 validateSubjectMembershipOwningType says the same thing
// from the abstract syntax side — the owningType must be a RequirementDefinition,
// RequirementUsage, CaseDefinition or CaseUsage — but the grammar settles it first,
// which is why this is by rule and not by constraint.
part def Vehicle {
	subject vehicle : Vehicle;
}
