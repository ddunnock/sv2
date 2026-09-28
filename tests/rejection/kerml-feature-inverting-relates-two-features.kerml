// REJECTED BY RULE. A FeatureInverting relates one featureInverted to one
// invertingFeature: each side is a single name or feature chain, never a list.
// KerML 8.2.4.3.6 — FeatureInverting = ( 'inverting' Identification? )?
//                       'inverse' ( [QualifiedName] | OwnedFeatureChain )
//                       'of' ( [QualifiedName] | OwnedFeatureChain )
//                       RelationshipBody
// rejects: PARSE-EXPECTED on: inverse Person::parents of Person::children, Person::offspring;
package Family {
	inverse Person::parents of Person::children, Person::offspring;
}
