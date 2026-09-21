// REJECTED BY RULE. AssertConstraintUsage is SysML's (8.2.2.20), reached through
// UsageElement, and KerML has no usages: a .kerml file is read against KerML's
// NamespaceBodyElement (ADR-0014). KerML asserts with an Invariant, `inv`, and `assert`
// is not a KerML keyword at all.
// KerML 8.2.3.4.1  — NamespaceBodyElement = NamespaceMember | AliasMember | Import
// KerML 8.2.5.7.4  — Invariant = FeaturePrefix 'inv' ( 'true' | 'false' )?
//                      FeatureDeclaration ValuePart? FunctionBody
// rejects: PARSE-UNEXPECTED on: assert constraint c;
assert constraint c;
