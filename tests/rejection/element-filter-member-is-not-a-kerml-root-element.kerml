// REJECTED BY RULE. An ElementFilterMember is admitted by KerML's PackageBody and by
// nothing else in KerML: it is not one of NamespaceBodyElement's three alternatives,
// and RootNamespace is NamespaceBodyElement*. So `filter` is well formed inside a
// KerML `package { ... }` and not at the root of a KerML file.
// KerML 8.2.3.4.1 — RootNamespace = NamespaceBodyElement*
//                   NamespaceBodyElement = NamespaceMember | AliasMember | Import
//                   PackageBody = ';'
//                               | '{' ( NamespaceBodyElement | ElementFilterMember )* '}'
//
// This is the case that keeps "which member does this body own" and "does this body
// admit a filter" as two questions. They give the same answer in SysML, where
// ElementFilterMember is a PackageBodyElement and so is admitted at the root too
// (8.2.2.5.1); they give different answers here. Deriving the second from the first
// would accept this file.
filter @Safety;
