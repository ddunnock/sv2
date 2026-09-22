// REJECTED BY ABSENCE. ActorMember is unimplemented, in a case body and a requirement body
// alike, so `actor` at item position is recovered over. 7.22.2's own FaultRecovery example
// writes one (receipt eb25a69f), as do the use cases of Annex A.
// SysML 8.2.2.22   — CaseBodyItem = ActionBodyItem | SubjectMember | ActorMember
//                                 | ObjectiveMember
// SysML 8.2.2.21.1 — ActorMember = MemberPrefix ActorUsage
//                    ActorUsage  = 'actor' UsageExtensionKeyword* Usage
// Remove this case when ActorMember lands; CaseBodyItem stays unmarked until it does.
// rejects: PARSE-UNEXPECTED on: actor engineer : Person;
case def FaultRecovery {
	subject system : AutomationSystem;
	actor engineer : Person;
}
