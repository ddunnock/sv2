// REJECTED BY ABSENCE. `calc` without `def` is a CalculationUsage, a different
// production, and it is not implemented.
// SysML 8.2.2.19 — CalculationUsage      = OccurrenceUsagePrefix 'calc'
//                                          ActionUsageDeclaration CalculationBody
//                  CalculationDefinition = OccurrenceDefinitionPrefix 'calc' 'def'
//                                          DefinitionDeclaration CalculationBody
// The two share every token before the `def`, so this is here for the reason
// tests/rejection/constraint-usage-is-not-a-constraint-definition.sysml is: a
// recogniser that looked only for `calc` would read a usage as a definition. The
// corpus writes the usage — `calc ms : MassSum { ... }` in
// vendor/corpus/sysml/src/examples/Simple Tests/CalculationTest.sysml — so removing
// this rejection is real work and not a formality.
calc c { a + b }
