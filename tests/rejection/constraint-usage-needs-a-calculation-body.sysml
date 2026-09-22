// REJECTED BY RULE. ConstraintUsage ends in CalculationBody -- `;` or a braced body -- and
// it is the last part of the production, not an optional one. The usage is recognised by
// `constraint` with no `def` after it, so it IS read and CalculationBody reports its
// absence.
// SysML 8.2.2.20  — ConstraintUsage = OccurrenceUsagePrefix 'constraint'
//                                     ConstraintUsageDeclaration CalculationBody
// SysML 8.2.2.19  — CalculationBody = ';' | '{' CalculationBodyPart '}'
// rejects: PARSE-EXPECTED on: }
part def P {
	constraint c
}
