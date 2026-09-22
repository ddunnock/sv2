// REJECTED BY RULE. RequirementUsage is SysML's (8.2.2.21.2), reached through
// UsageElement, and KerML has no usages: a .kerml file is read against KerML's
// NamespaceBodyElement (ADR-0014). `requirement` is not a KerML keyword at all.
// KerML 8.2.3.4.1  — NamespaceBodyElement = NamespaceMember | AliasMember | Import
// rejects: PARSE-UNEXPECTED on: requirement r;
requirement r;
