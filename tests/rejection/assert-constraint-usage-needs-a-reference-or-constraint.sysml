// REJECTED BY RULE. After `assert` and an optional `not`, one of the two alternatives is
// required: the constraint asserted by reference, or `constraint` and a declaration.
// Neither is optional, so a bare `assert;` asserts nothing.
// SysML 8.2.2.20  — AssertConstraintUsage = OccurrenceUsagePrefix 'assert' 'not'?
//                     ( OwnedReferenceSubsetting FeatureSpecializationPart?
//                     | 'constraint' ConstraintUsageDeclaration ) CalculationBody
// SysML 7.20.3    — "the constraint to be asserted is identified by giving a qualified name
//                   or feature chain immediately after the assert keyword" (receipt 44d633db)
// rejects: PARSE-EXPECTED on: assert;
part def P {
	assert;
}
