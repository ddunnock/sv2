// REJECTED BY RULE. An objective is declared "as a requirement usage ..., but using the
// keyword objective instead of requirement" (7.22.2, receipt eb25a69f), and it is a CASE's
// member: CaseBodyItem alone names ObjectiveMember. A requirement body reaches
// DefinitionBodyItem and its own six members, and objective is none of them.
// SysML 8.2.2.22   — CaseBodyItem = ActionBodyItem | SubjectMember | ActorMember
//                                 | ObjectiveMember
// SysML 8.2.2.21.1 — RequirementBodyItem = DefinitionBodyItem | SubjectMember
//                                 | RequirementConstraintMember | … | StakeholderMember
// rejects: PARSE-UNEXPECTED on: objective o;
requirement def R {
	objective o;
}
