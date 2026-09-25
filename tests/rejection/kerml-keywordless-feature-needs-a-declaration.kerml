// REJECTED BY RULE. A feature written with neither `feature` nor a `#` in its place is
// declared by its FeatureDeclaration alone, which is therefore required: a prefix and a
// value say nothing about which feature is meant.
// KerML 8.2.4.3.1 — Feature = ( FeaturePrefix ( 'feature' | PrefixMetadataMember )
//                               FeatureDeclaration?
//                             | ( EndFeaturePrefix | BasicFeaturePrefix )
//                               FeatureDeclaration )
//                             ValuePart? TypeBody
// rejects: PARSE-UNEXPECTED on: composite = e;
struct Vehicle {
	composite = e;
}
