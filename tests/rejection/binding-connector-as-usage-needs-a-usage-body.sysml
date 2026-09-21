// REJECTED BY RULE. BindingConnectorAsUsage ends in UsageBody -- `;` or a braced body --
// and it is the last part of the production, not an optional one. The binding is
// recognised by its reserved `bind`, so it IS read and UsageBody reports its absence.
// SysML 8.2.2.13.2  — BindingConnectorAsUsage = UsagePrefix ( 'binding' UsageDeclaration )?
//                                               'bind' ConnectorEndMember
//                                               '=' ConnectorEndMember UsageBody
// SysML 8.2.2.6.2   — UsageBody = DefinitionBody
// SysML 8.2.2.6.1   — DefinitionBody = ';' | '{' DefinitionBodyItem* '}'
// rejects: PARSE-EXPECTED on: }
part def P {
	bind a = b
}
