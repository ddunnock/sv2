// REJECTED BY RULE. Each '.' in a FeatureChain must be followed by a link.
// KerML 8.2.4.3.1 — ChainingPart         = 'chains' ( OwnedFeatureChaining | FeatureChain )
// KerML 8.2.4.3.5 — FeatureChain         = OwnedFeatureChaining
//                                          ( '.' OwnedFeatureChaining )+
//                   OwnedFeatureChaining = chainingFeature = [QualifiedName]
// A QualifiedName opens on a NAME, and `;` is not one.
// rejects: PARSE-EXPECTED on: feature cousins chains parents.siblings.;
package P {
	feature cousins chains parents.siblings.;
}
