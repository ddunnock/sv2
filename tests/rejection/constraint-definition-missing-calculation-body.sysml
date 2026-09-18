// REJECTED BY RULE. CalculationBody is not optional, and neither of its two forms is
// written here.
// SysML 8.2.2.20 — ConstraintDefinition = OccurrenceDefinitionPrefix 'constraint' 'def'
//                                         DefinitionDeclaration CalculationBody
// SysML 8.2.2.19 — CalculationBody      = ';' | '{' CalculationBodyPart '}'
// The sibling of tests/rejection/requirement-definition-missing-requirement-body.sysml.
constraint def C
