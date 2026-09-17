// REJECTION BY ABSENCE, not by rule. BodyExpression is the BaseExpression
// alternative this layer deliberately stops before: it reaches ExpressionBody, and
// ExpressionBody in SysML reads CalculationBody, which reaches most of the
// language. That is why SelectExpression and CollectExpression are absent too -
// both take a BodyArgumentMember.
// KerML 8.2.5.8.3 - BodyExpression = ownedRelationship += ExpressionBodyMember
// KerML 8.2.5.8.3 - ExpressionBody = '{' FunctionBodyPart '}'
// SysML 8.2.2.17.3 - ExpressionBody = CalculationBody
// The SysML reading of ExpressionBody is one of the two recorded scope deviations
// (ADR-0015); see SYSML_BOUNDARY in .claude/scripts/_grammar.py.
// Remove this case when BodyExpression lands.
attribute x = {in y; y};
