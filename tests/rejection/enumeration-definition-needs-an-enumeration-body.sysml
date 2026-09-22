// REJECTED BY RULE. EnumerationDefinition ends in EnumerationBody -- `;` or a braced body
// -- and it is the last part of the production, not an optional one. `enum def` is
// recognised, so the definition IS read and EnumerationBody reports its absence.
// SysML 8.2.2.8 — EnumerationDefinition = DefinitionExtensionKeyword* 'enum' 'def'
//                                         DefinitionDeclaration EnumerationBody
//                 EnumerationBody = ';' | '{' ( AnnotatingMember | EnumerationUsageMember )* '}'
// rejects: PARSE-EXPECTED on: }
package P {
	enum def E
}
