// REJECTED BY ABSENCE. CalculationDefinition shares ConstraintDefinition's shape and
// its body, and is still absent: nothing dispatches to it.
// SysML 8.2.2.19 — CalculationDefinition = OccurrenceDefinitionPrefix 'calc' 'def'
//                                          DefinitionDeclaration CalculationBody
// SysML 8.2.2.20 — ConstraintDefinition  = OccurrenceDefinitionPrefix 'constraint' 'def'
//                                          DefinitionDeclaration CalculationBody
// The two differ in one keyword, so this case exists to keep that from being read as a
// reason the parser already handles `calc`. It does not. Every `calc def` body in the
// corpus also writes `return`, so implementing the keyword alone would buy nothing.
calc def C { a }
