// REJECTED BY RULE. A TypeFeatureMember owns a FeatureElement: a classifier is a
// NonFeatureElement and is owned through a NonFeatureMember, with no `member`.
// KerML 8.2.4.1.6 — TypeFeatureMember = MemberPrefix 'member' FeatureElement
// KerML 8.2.3.4.3 — FeatureElement    = Feature | Step | Expression | BooleanExpression
//                                     | Invariant | Connector | BindingConnector
//                                     | Succession | Flow | SuccessionFlow
// rejects: PARSE-EXPECTED on: member class Driver;
struct Car {
	member class Driver;
}
