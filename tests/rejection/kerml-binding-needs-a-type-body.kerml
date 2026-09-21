// REJECTED BY RULE. BindingConnector ends in TypeBody -- `;` or a braced body -- and it is
// the last part of the production, not an optional one. The binding is recognised by its
// reserved keyword, so it IS read and TypeBody reports its absence.
// KerML 8.2.5.5.2   — BindingConnector = FeaturePrefix 'binding' BindingConnectorDeclaration TypeBody
// KerML 8.2.4.1.1   — TypeBody = ';' | '{' TypeBodyElement* '}'
// rejects: PARSE-EXPECTED on: }
class C {
	binding a = b
}
