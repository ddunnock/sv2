// REJECTED BY RULE. SuccessionDeclaration's ends come as a pair or not at all: `first`
// and its source end are never written without `then` and the target end after them.
// A succession that names only where it starts is no production.
// KerML 8.2.5.5.3 — SuccessionDeclaration =
//                       FeatureDeclaration ( 'first' ConnectorEndMember
//                                            'then' ConnectorEndMember )?
//                     | 'all'? ( 'first'? ConnectorEndMember 'then' ConnectorEndMember )?
// rejects: PARSE-EXPECTED on: succession first a;
class C {
	succession first a;
}
