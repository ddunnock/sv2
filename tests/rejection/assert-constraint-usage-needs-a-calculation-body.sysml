// REJECTED BY RULE. AssertConstraintUsage ends in CalculationBody -- `;` or a braced body
// -- and it is the last part of the production, not an optional one. The assertion is
// recognised by its reserved `assert`, so it IS read and CalculationBody reports its
// absence.
// SysML 8.2.2.20  — AssertConstraintUsage = OccurrenceUsagePrefix 'assert' 'not'?
//                     ( OwnedReferenceSubsetting FeatureSpecializationPart?
//                     | 'constraint' ConstraintUsageDeclaration ) CalculationBody
// SysML 8.2.2.19  — CalculationBody = ';' | '{' CalculationBodyPart '}'
// rejects: PARSE-EXPECTED on: }
part def P {
	assert constraint c
}
