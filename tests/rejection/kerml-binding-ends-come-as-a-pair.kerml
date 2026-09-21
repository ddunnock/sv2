// REJECTED BY RULE. BindingConnectorDeclaration's ends come as a pair or not at all: `of`
// and its source end are never written without `=` and the target. Here the first
// alternative's FeatureDeclaration names the binding, and its optional ends group is
// opened by `of` and left unfinished.
// KerML 8.2.5.5.2   — BindingConnectorDeclaration =
//                         FeatureDeclaration ( 'of' ConnectorEndMember '=' ConnectorEndMember )?
//                       | 'all'? ( 'of'? ConnectorEndMember '=' ConnectorEndMember )?
// rejects: PARSE-EXPECTED on: binding b of a;
class C {
	binding b of a;
}
