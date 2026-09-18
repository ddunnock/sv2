// REJECTED BY ABSENCE. `constraint` without `def` is a ConstraintUsage, a different
// production, and it is not implemented.
// SysML 8.2.2.20 — ConstraintUsage      = OccurrenceUsagePrefix 'constraint'
//                                         ConstraintUsageDeclaration CalculationBody
//                  ConstraintDefinition = OccurrenceDefinitionPrefix 'constraint' 'def'
//                                         DefinitionDeclaration CalculationBody
// The two share every token before the `def`, so this is here for the reason
// tests/rejection/requirement-usage-is-not-a-requirement-definition.sysml is: a
// recogniser that looked only for `constraint` would read a usage as a definition.
// ConstraintUsage is what `require constraint { ... }` needs, so removing this
// rejection is the next step after this one.
constraint c { a <= b }
