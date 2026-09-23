// REJECTED BY RULE. A select expression's only argument is a body: SelectExpression
// writes BodyArgumentMember after '.?' and offers no ArgumentList, unlike
// FunctionOperationExpression beside it. The corpus writes the body form:
// `subcomponents.totalMass.?{in p:>ISQ::mass; p >= minMass}`
// (training/29. Expressions/MassRollup2.sysml:18).
// KerML 8.2.5.8.2 - SelectExpression =
//                     ownedRelationship += PrimaryArgumentMember '.?'
//                     ownedRelationship += BodyArgumentMember
// rejects: PARSE-EXPECTED on: attribute x = things.?(a);
attribute x = things.?(a);
