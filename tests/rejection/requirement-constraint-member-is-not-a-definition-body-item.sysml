// REJECTED BY RULE. A RequirementConstraintMember is an alternative of
// RequirementBodyItem and of no other body this parser reads.
// SysML 8.2.2.6.1  — DefinitionBodyItem  = DefinitionMember | VariantUsageMember
//                                        | NonOccurrenceUsageMember
//                                        | SourceSuccessionMember? OccurrenceUsageMember
//                                        | AliasMember | Import
// SysML 8.2.2.21.1 — RequirementBodyItem = DefinitionBodyItem | SubjectMember
//                                        | RequirementConstraintMember | ...
// The sibling of tests/rejection/subject-member-is-not-a-definition-body-item.sysml.
// A calculation body does not admit one either, although it is the body a
// `require constraint` ends in: CalculationBodyItem reaches NonBehaviorBodyItem
// (8.2.2.17.1), which has no requirement members at all.
part def Vehicle {
	require constraint { massActual <= massReqd }
}
