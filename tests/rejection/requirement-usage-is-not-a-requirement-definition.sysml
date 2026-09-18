// REJECTED BY ABSENCE. `requirement` without `def` is a RequirementUsage, a different
// production, and it is not implemented.
// SysML 8.2.2.21.2 — RequirementUsage    = OccurrenceUsagePrefix 'requirement'
//                                          CalculationUsageDeclaration RequirementBody
// SysML 8.2.2.21.1 — RequirementDefinition = OccurrenceDefinitionPrefix 'requirement'
//                                            'def' DefinitionDeclaration RequirementBody
// Here so that the recogniser cannot quietly drop the `def`: the two productions share
// every token before it, and a check that looked only for `requirement` would read a
// usage as a definition and lose the distinction the abstract syntax rests on.
// Removing this rejection means implementing RequirementUsage.
requirement r;
