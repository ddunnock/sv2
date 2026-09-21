// REJECTED BY ABSENCE. `assert satisfy` opens SatisfyRequirementUsage, not an
// AssertConstraintUsage, and SatisfyRequirementUsage is not implemented. `satisfy` is
// reserved, so it cannot be the QualifiedName of AssertConstraintUsage's reference
// alternative, and the assertion's recogniser declines it; implementing
// SatisfyRequirementUsage removes this file.
// SysML 8.2.2.21.2 — SatisfyRequirementUsage = OccurrenceUsagePrefix 'assert' ( 'not' )?
//                      'satisfy' ...
// SysML 8.2.2.20   — AssertConstraintUsage = OccurrenceUsagePrefix 'assert' 'not'?
//                      ( OwnedReferenceSubsetting FeatureSpecializationPart? | ... ) ...
// rejects: PARSE-UNEXPECTED on: assert satisfy r;
part def P {
	assert satisfy r;
}
