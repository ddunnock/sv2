// REJECTED BY RULE. A stakeholder is a parameter of a requirement: StakeholderMember is one
// of RequirementBodyItem's own alternatives, and CaseBodyItem, which has ActorMember, does
// not have it.
// SysML 8.2.2.21.1 — RequirementBodyItem = ... | ActorMember | StakeholderMember
// SysML 8.2.2.22   — CaseBodyItem = ActionBodyItem | SubjectMember | ActorMember
//                      | ObjectiveMember
// The diagnostic is the result-expression reader's ("expected an expression"): a case body
// hands a reserved word it has no item for to its result expression, pending decision
// reserved-word-result-expression. The token it falls on is the rule's.
// rejects: PARSE-EXPECTED on: stakeholder driver : Person;
case def BrakingCase {
	stakeholder driver : Person;
}
