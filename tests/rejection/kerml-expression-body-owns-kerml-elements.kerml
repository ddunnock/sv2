// REJECTED BY RULE. A KerML ExpressionBody owns KerML's FunctionBodyPart: TypeBodyElements,
// ReturnFeatureMembers and a result expression. SysML's usages are not among them; SysML
// reads its own bodies as CalculationBodies, by deviation ExpressionBody, in .sysml alone.
// KerML 8.2.5.8.3 — ExpressionBody   = '{' FunctionBodyPart '}'
// KerML 8.2.5.7.1 — FunctionBodyPart = ( TypeBodyElement | ReturnFeatureMember )*
//                                      ( ResultExpressionMember )?
// rejects: PARSE-EXPECTED on: c = x->collect { part p; 1 };
package Expressions {
	c = x->collect { part p; 1 };
}
