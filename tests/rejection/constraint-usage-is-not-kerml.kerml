// REJECTED BY RULE. ConstraintUsage is SysML's (8.2.2.20), reached through UsageElement,
// and KerML has no usages: a .kerml file is read against KerML's NamespaceBodyElement
// (ADR-0014). KerML declares a boolean expression step with `bool`, and `constraint` is
// not a KerML keyword at all.
// KerML 8.2.3.4.1  — NamespaceBodyElement = NamespaceMember | AliasMember | Import
// KerML 8.2.5.7.4  — BooleanExpression = FeaturePrefix 'bool' FeatureDeclaration ValuePart?
//                      FunctionBody
// rejects: PARSE-UNEXPECTED on: constraint c { a <= b }
constraint c { a <= b }
