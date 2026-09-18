// REJECTED BY RULE. `action def` is an ActionDefinition and `action` alone is an
// ActionUsage; both are implemented, and the `def` is the only thing between them. What
// is rejected here is a declaration that claims to be both.
// SysML 8.2.2.17.1 — ActionDefinition = OccurrenceDefinitionPrefix 'action' 'def'
//                                       DefinitionDeclaration ActionBody
// SysML 8.2.2.17.2 — ActionUsage      = OccurrenceUsagePrefix 'action'
//                                       ActionUsageDeclaration ActionBody
// `at_action_usage` requires that NO `def` follow the keyword, exactly as every one of
// SIMPLE_USAGES does. Without that test an `action def` would be read as a usage named
// `def`, and the definition would never be reached.
package P {
	action def def Brake;
}
