// REJECTED BY RULE. CalculationBody is not optional, and neither of its two forms is
// written here.
// SysML 8.2.2.19 — CalculationDefinition = OccurrenceDefinitionPrefix 'calc' 'def'
//                                          DefinitionDeclaration CalculationBody
// SysML 8.2.2.19 — CalculationBody       = ';' | '{' CalculationBodyPart '}'
// The sibling of tests/rejection/constraint-definition-missing-calculation-body.sysml,
// over the body the two productions share.
calc def C
