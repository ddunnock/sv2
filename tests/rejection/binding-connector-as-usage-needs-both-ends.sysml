// REJECTED BY RULE. A binding relates exactly two features: `'bind' ConnectorEndMember
// '=' ConnectorEndMember`, neither optional. With one end written, the `=` is missing.
// SysML 8.2.2.13.2  — BindingConnectorAsUsage = UsagePrefix ( 'binding' UsageDeclaration )?
//                                               'bind' ConnectorEndMember
//                                               '=' ConnectorEndMember UsageBody
// SysML 7.13.3      — "The two related features of a binding are identified after its
//                     declaration part and before its body, following the keyword bind
//                     and separated by the symbol =." (receipt 6db87b41)
// rejects: PARSE-EXPECTED on: bind a;
part def P {
	bind a;
}
