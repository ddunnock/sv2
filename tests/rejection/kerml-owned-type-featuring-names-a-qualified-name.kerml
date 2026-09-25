// REJECTED BY RULE. A featuring type is named by a qualified name alone. Unlike the
// targets of the relationship parts, an OwnedTypeFeaturing has no feature chain
// alternative.
// KerML 8.2.4.3.7 — OwnedTypeFeaturing = featuringType = [QualifiedName]
// KerML 8.2.4.3.1 — TypeFeaturingPart  = 'featured' 'by' OwnedTypeFeaturing
//                                        ( ',' OwnedTypeFeaturing )*
// rejects: PARSE-EXPECTED on: feature coincident : Occurrence [1] featured by universalEternity.portions;
package Moments {
	feature coincident : Occurrence [1] featured by universalEternity.portions;
}
