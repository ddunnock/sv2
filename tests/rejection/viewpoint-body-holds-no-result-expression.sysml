// REJECTED BY RULE. A viewpoint is declared as a kind of requirement (7.26.3), so its
// body is a RequirementBody, whose items are all members: unlike a constraint's
// CalculationBody it ends in no result expression. A viewpoint's constraints are written
// as `require constraint { ... }` members, as 7.26.3's own example does.
// SysML 8.2.2.26.3 - ViewpointUsage = OccurrenceUsagePrefix 'viewpoint'
//                                     ConstraintUsageDeclaration RequirementBody
// SysML 8.2.2.21.1 - RequirementBody = ';' | '{' RequirementBodyItem* '}'
// rejects: PARSE-EXPECTED on: viewpoint vp { x > 0 }
part p {
	viewpoint vp { x > 0 }
}
