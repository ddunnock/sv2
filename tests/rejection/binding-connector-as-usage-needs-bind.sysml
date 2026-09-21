// REJECTED BY RULE. The `binding` keyword and its UsageDeclaration are the optional part
// of BindingConnectorAsUsage; `'bind' ConnectorEndMember '=' ConnectorEndMember` is not.
// A binding declared with no ends is no production -- the KerML BindingConnector, whose
// ends are optional (8.2.5.5.2), is not reachable from a .sysml file (ADR-0014).
// SysML 8.2.2.13.2  — BindingConnectorAsUsage = UsagePrefix ( 'binding' UsageDeclaration )?
//                                               'bind' ConnectorEndMember
//                                               '=' ConnectorEndMember UsageBody
// rejects: PARSE-EXPECTED on: binding b;
part def P {
	binding b;
}
