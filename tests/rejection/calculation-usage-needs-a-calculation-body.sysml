// REJECTED BY RULE. CalculationUsage ends in CalculationBody -- `;` or a braced body -- and
// it is the last part of the production, not an optional one. The usage is recognised by
// `calc` with no `def` after it, so it IS read and CalculationBody reports its absence.
// SysML 8.2.2.19  — CalculationUsage = OccurrenceUsagePrefix 'calc'
//                                      ActionUsageDeclaration CalculationBody
// SysML 8.2.2.19  — CalculationBody = ';' | '{' CalculationBodyPart '}'
// rejects: PARSE-EXPECTED on: }
part def P {
	calc c
}
