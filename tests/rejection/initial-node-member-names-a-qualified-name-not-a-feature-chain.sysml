// REJECTED BY RULE. InitialNodeMember names its feature by [QualifiedName], and a feature
// chain is not one. The productions that do take a chain after `first` —
// SuccessionAsUsage and GuardedSuccession — both go on to `then`, so `first a.b;` is
// none of the three.
// SysML 8.2.2.17.1  — InitialNodeMember = MemberPrefix 'first' memberFeature = [QualifiedName]
//                                         RelationshipBody
// KerML 8.2.3.4.1   — QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME
// SysML 8.2.2.13.3  — SuccessionAsUsage = … 'first' ConnectorEndMember 'then' …
// SysML 8.2.2.17.8  — GuardedSuccession = … 'first' FeatureChainMember
//                                         GuardExpressionMember 'then' …
action def A {
	first a.b;
}
