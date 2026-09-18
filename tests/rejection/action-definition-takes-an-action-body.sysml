// REJECTED BY ABSENCE, not by rule. Fourteen of the twenty-two productions with a
// `def` keyword end in a specialised body rather than in Definition, and not one of
// those bodies is implemented. ActionDefinition is the most common in the corpus.
// SysML 8.2.2.17 — ActionDefinition = OccurrenceDefinitionPrefix 'action' 'def'
//                                     DefinitionDeclaration ActionBody
// SysML 8.2.2.6.1 — Definition = DefinitionDeclaration DefinitionBody
// The two bodies are different productions: ActionBody admits action nodes and
// successions that a DefinitionBody does not. Putting `action` in the table would
// accept the declaration and then read its body against the wrong rule.
action def Brake;
