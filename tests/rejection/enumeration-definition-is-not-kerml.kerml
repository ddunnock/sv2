// REJECTED BY RULE. EnumerationDefinition is SysML's (8.2.2.8), reached through
// DefinitionElement, and a .kerml file is read against KerML's NamespaceBodyElement
// (ADR-0014). KerML declares a data type with `datatype`; `enum` is not a KerML keyword
// at all.
// KerML 8.2.3.4.1 — NamespaceBodyElement = NamespaceMember | AliasMember | Import
// rejects: PARSE-UNEXPECTED on: enum def E;
enum def E;
