// REJECTED BY RULE. `verify` belongs to a verification case's OBJECTIVE, not to the case:
// "the objective of a verification case may also have requirement verification usages"
// (7.24.2, receipt d518fc8c). RequirementVerificationMember is a RequirementBodyItem, and
// the objective's body is a RequirementBody; CaseBodyItem does not reach it.
// SysML 8.2.2.21.1 — RequirementBodyItem = DefinitionBodyItem | SubjectMember | …
//                                        | RequirementVerificationMember | …
// SysML 8.2.2.22   — CaseBodyItem = ActionBodyItem | SubjectMember | ActorMember
//                                 | ObjectiveMember
// Reported by the result-expression reader, as any reserved word a case body has no item
// for is today: `verify` is not an expression either, which is why it is rejected.
// rejects: PARSE-EXPECTED on: verify vehicleMassRequirement;
verification def VehicleMassTest {
	subject testVehicle : Vehicle;
	verify vehicleMassRequirement;
}
