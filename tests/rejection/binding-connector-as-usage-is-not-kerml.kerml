// REJECTED BY RULE. BindingConnectorAsUsage is SysML's (8.2.2.13.2), reached through
// UsageElement, and KerML has no usages: a .kerml file is read against KerML's
// NamespaceBodyElement (ADR-0014). KerML writes its binding as BindingConnector,
// `binding ... of a = b`, over FeaturePrefix; `bind` is not a KerML keyword at all.
// KerML 8.2.3.4.1   — NamespaceBodyElement = NamespaceMember | AliasMember | Import
// KerML 8.2.5.5.2   — BindingConnector = FeaturePrefix 'binding' BindingConnectorDeclaration TypeBody
// rejects: PARSE-UNEXPECTED on: bind a = b;
bind a = b;
