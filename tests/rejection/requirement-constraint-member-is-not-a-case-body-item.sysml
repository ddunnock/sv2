// REJECTED BY RULE. `require` and `assume` belong to a requirement body, and a case's
// requirement is its OBJECTIVE, whose body is a RequirementBody: the corpus writes them
// there (training/33. Analysis/Analysis Case Definition Example.sysml:38-44). CaseBodyItem
// reaches SubjectMember and not RequirementConstraintMember, so directly in a case body the
// line is neither an item nor the result expression, which cannot open on a keyword.
// SysML 8.2.2.22   — CaseBodyItem = ActionBodyItem | SubjectMember | ActorMember
//                                 | ObjectiveMember
// SysML 8.2.2.21.1 — RequirementConstraintMember = MemberPrefix? RequirementKind
//                                 RequirementConstraintUsage
// rejects: PARSE-EXPECTED on: require constraint { x > 0 }
analysis def A {
	require constraint { x > 0 }
}
