// REJECTED BY RULE. An actor is a parameter of a requirement or a case: "declared using
// the keywords actor and stakeholder rather than explicitly declaring their direction"
// (7.21.2, receipt 021b9219). Only RequirementBodyItem and CaseBodyItem name ActorMember;
// a part definition's body reaches DefinitionBodyItem, which does not.
// SysML 8.2.2.21.1 — ActorMember = MemberPrefix ActorUsage
// SysML 8.2.2.22   — CaseBodyItem = ActionBodyItem | SubjectMember | ActorMember
//                                 | ObjectiveMember
// SysML 8.2.2.6.1  — DefinitionBodyItem has no ActorMember alternative.
// rejects: PARSE-UNEXPECTED on: actor driver : Person;
part def Vehicle {
	actor driver : Person;
}
