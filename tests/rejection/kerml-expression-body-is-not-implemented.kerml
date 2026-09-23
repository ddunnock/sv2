// REJECTED BY ABSENCE, not by rule. KerML's ExpressionBody is '{' FunctionBodyPart '}',
// and FunctionBodyPart@kerml is unimplemented. SysML reads the same braces as its
// CalculationBody by deviation ExpressionBody, and that reading is SysML's alone, so a
// .kerml body is reported rather than read as SysML's. The corpus writes this:
// vendor/corpus/kerml/src/examples/Simple Tests/Expressions.kerml:15,
// `c = x->collect {in xx; xx + 1};`.
// KerML 8.2.5.8.3 - BodyExpression = ownedRelationship += ExpressionBodyMember
//                   ExpressionBody = '{' FunctionBodyPart '}'
// Remove this case when FunctionBodyPart@kerml lands.
// rejects: PARSE-EXPECTED on: feature c = x->collect {in xx; xx + 1};
package P {
	feature c = x->collect {in xx; xx + 1};
}
