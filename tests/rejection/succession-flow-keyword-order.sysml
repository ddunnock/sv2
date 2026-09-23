// REJECTED BY RULE. A succession flow is declared "using the keyword succession flow"
// (7.16.2, receipt 13d6f883): `succession`, then `flow`. Reversed, `flow` opens a
// FlowUsage whose FlowDeclaration cannot begin with the reserved `succession`.
// SysML 8.2.2.16 — SuccessionFlowUsage = OccurrenceUsagePrefix 'succession' 'flow'
//                                        FlowDeclaration DefinitionBody
// rejects: PARSE-EXPECTED on: flow succession focus.image to shoot.image;
part def P {
	flow succession focus.image to shoot.image;
}
