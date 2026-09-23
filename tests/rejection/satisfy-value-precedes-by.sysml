// REJECTED BY RULE. A satisfy requirement usage's ValuePart comes before its `by`
// subject, not after it.
// SysML 8.2.2.21.2 — SatisfyRequirementUsage = ... ValuePart?
//                      ( 'by' ownedRelationship += SatisfactionSubjectMember )?
//                      RequirementBody
// rejects: PARSE-EXPECTED on: satisfy vehicleMaximumMass by vehicle1 = true;
part def Context {
	part vehicle1 : Vehicle;
	satisfy vehicleMaximumMass by vehicle1 = true;
}
