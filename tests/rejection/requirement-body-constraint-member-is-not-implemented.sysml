// REJECTED BY ABSENCE, not by rule. A RequirementConstraintMember is a
// RequirementBodyItem and this text is well-formed SysML; the member is not implemented.
// SysML 8.2.2.21.1 — RequirementConstraintMember = RequirementConstraintKind
//                                                  ConstraintUsageElement
//                    RequirementConstraintKind    = 'assume' | 'require'
// Both kinds are here because `assume` and `require` are one production with a kind
// flag, so a rejection of only one of them would not say which half is missing.
// Removing this rejection means implementing RequirementConstraintMember.
requirement def R {
	assume constraint { a > 0 }
	require constraint { a <= b }
}
