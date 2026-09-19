// REJECTED BY RULE. FlowDeclaration's first alternative writes its ends as a pair:
// `'from' FlowEndMember 'to' FlowEndMember`, optional together. A `from` end with no
// `to` end is neither that group nor the second alternative, which also needs `to`.
// SysML 8.2.2.16  — FlowDeclaration = UsageDeclaration ValuePart?
//                                     ( 'of' FlowPayloadFeatureMember )?
//                                     ( 'from' FlowEndMember 'to' FlowEndMember )?
//                                   | FlowEndMember 'to' FlowEndMember
part def P {
	flow from a.b;
}
