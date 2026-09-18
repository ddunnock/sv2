// REJECTED BY ABSENCE. `action` without `def` is an ActionUsage, a different production,
// and it is not implemented.
// SysML 8.2.2.17.2 — ActionUsage      = OccurrenceUsagePrefix 'action'
//                                       ActionUsageDeclaration ActionBody
//                    PerformActionUsage = OccurrenceUsagePrefix 'perform'
//                                       PerformActionUsageDeclaration ActionBody
// SysML 8.2.2.17.1 — ActionDefinition = OccurrenceDefinitionPrefix 'action' 'def'
//                                       DefinitionDeclaration ActionBody
// Here for the reason the requirement and constraint usages have such a file: the two
// share every token before the `def`, and a recogniser that looked only for `action`
// would read a usage as a definition.
// These two are also what the corpus is ACTUALLY blocked on. Of the 46 files that write
// an `action def` and still fail, 18 fail first on `action` and 6 on `perform` — both
// usages — against one on `first`. ActionBody already exists, so removing this rejection
// is a declaration, not the control-flow layer.
package P {
	action brake;
	perform action stop;
}
