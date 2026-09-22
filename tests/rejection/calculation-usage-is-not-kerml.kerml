// REJECTED BY RULE. CalculationUsage is SysML's (8.2.2.19), reached through UsageElement,
// and KerML has no usages: a .kerml file is read against KerML's NamespaceBodyElement
// (ADR-0014). KerML declares an expression step with `expr`, and `calc` is not a KerML
// keyword at all.
// KerML 8.2.3.4.1  — NamespaceBodyElement = NamespaceMember | AliasMember | Import
// KerML 8.2.5.7.2  — Expression = FeaturePrefix 'expr' FeatureDeclaration ValuePart? FunctionBody
// rejects: PARSE-UNEXPECTED on: calc c { a + b }
calc c { a + b }
