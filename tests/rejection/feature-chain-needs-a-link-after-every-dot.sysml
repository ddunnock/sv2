// REJECTED BY RULE. Each '.' in an OwnedFeatureChain must be followed by a link.
// SysML 8.2.2.6.5 — OwnedFeatureChain    = ownedRelationship += OwnedFeatureChaining
//                                          ( '.' ownedRelationship
//                                              += OwnedFeatureChaining )+
//                   OwnedFeatureChaining = chainingFeature = [QualifiedName]
// A QualifiedName opens on a NAME, and `;` is not one. The same rule the expression
// layer's chain has -- `a.` is not an expression either -- stated by a different
// production in a different position.
package P {
	part p :>> a.;
}
