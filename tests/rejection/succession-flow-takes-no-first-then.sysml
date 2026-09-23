// REJECTED BY RULE. A succession flow names its ends as a flow does, `from ... to ...`
// or `... to ...`, not as a SuccessionAsUsage does with `first ... then ...`.
// SysML 8.2.2.16   — SuccessionFlowUsage = OccurrenceUsagePrefix 'succession' 'flow'
//                                          FlowDeclaration DefinitionBody
// SysML 8.2.2.13.3 — SuccessionAsUsage = UsagePrefix ( 'succession' UsageDeclaration )?
//                                        'first' ConnectorEndMember 'then' ConnectorEndMember
//                                        UsageBody
// rejects: PARSE-EXPECTED on: succession flow first focus then shoot;
part def P {
	succession flow first focus then shoot;
}
