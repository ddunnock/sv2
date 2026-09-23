// REJECTED BY DEVIATION. SysML states no ExpressionBody; deviation ExpressionBody
// (follow_xtext) reads it as SysML's CalculationBody, and decision
// expression-body-semicolon (2026-09-23) narrows that to the braced alternative. Every
// expression body in the corpus is braced -- `rearWheelChoice->forAll {in ref w; ...}`
// (validation/07-Variant Configuration/7b-Variant Configurations.sysml:127) -- and none
// is a bare `;`, which taken literally would make `a->f;` a complete operation and
// this line an attribute: the first `;` the body, the second the attribute's own.
// KerML 8.2.5.8.2 - FunctionOperationExpression = PrimaryArgumentMember '->'
//                     InstantiatedTypeMember ( BodyArgumentMember
//                     | FunctionReferenceArgumentMember | ArgumentList ) EmptyResultMember
// SysML 8.2.2.19  - CalculationBody = ';' | '{' CalculationBodyPart '}'
// rejects: PARSE-EXPECTED on: attribute x = a->f;;
attribute x = a->f;;
