// REJECTED BY RULE. There is no succession flow definition: a succession flow usage is
// defined by flow definitions (7.16.2, receipt 13d6f883), and FlowDefinition writes
// `flow def` alone. `succession flow` is a usage, and `def` cannot follow its keywords.
// SysML 8.2.2.16 — FlowDefinition      = OccurrenceDefinitionPrefix 'flow' 'def' Definition
//                  SuccessionFlowUsage = OccurrenceUsagePrefix 'succession' 'flow'
//                                        FlowDeclaration DefinitionBody
// rejects: PARSE-EXPECTED on: succession flow def ImageFlow;
succession flow def ImageFlow;
