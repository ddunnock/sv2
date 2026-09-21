// REJECTED BY RULE. SuccessionAsUsage ends in UsageBody -- `;` or a braced body -- and it
// is the last part of the production, not an optional one. The succession is recognised
// by the `then` after its source end, so it IS read and UsageBody reports its absence.
// SysML 8.2.2.13.3  — SuccessionAsUsage = UsagePrefix ( 'succession' UsageDeclaration )?
//                                         'first' ConnectorEndMember
//                                         'then' ConnectorEndMember UsageBody
// SysML 8.2.2.6.2   — UsageBody = DefinitionBody
// SysML 8.2.2.6.1   — DefinitionBody = ';' | '{' DefinitionBodyItem* '}'
// rejects: PARSE-EXPECTED on: }
part def P {
	first a then b
}
