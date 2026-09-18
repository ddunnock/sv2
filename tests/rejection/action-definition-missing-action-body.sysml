// REJECTED BY RULE. ActionBody is not optional, and neither of its two forms is here.
// SysML 8.2.2.17.1 — ActionDefinition = OccurrenceDefinitionPrefix 'action' 'def'
//                                       DefinitionDeclaration ActionBody
//                    ActionBody       = ';' | '{' ActionBodyItem* '}'
// The sibling of requirement-definition-missing-requirement-body.sysml and
// constraint-definition-missing-calculation-body.sysml.
action def Brake
