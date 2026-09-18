// REJECTED BY RULE. A ReturnParameterMember is an alternative of CalculationBodyItem
// and of nothing else that is implemented, so a `return` in a definition body is not a
// member this grammar has.
// SysML 8.2.2.19   — CalculationBodyItem = ActionBodyItem | ReturnParameterMember
// SysML 8.2.2.6.1  — DefinitionBodyItem  = DefinitionMember | VariantUsageMember
//                                        | NonOccurrenceUsageMember
//                                        | SourceSuccessionMember? OccurrenceUsageMember
//                                        | AliasMember | Import
// The sibling of tests/rejection/element-filter-member-is-not-a-definition-body-item.sysml:
// what a body ADMITS is a separate question from what it owns its members through, and
// deriving the first from the second is what this file exists to catch.
part def V {
	return x;
}
