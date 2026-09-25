// REJECTED BY RULE. FeaturePrefix writes its PrefixMetadataMembers after the Basic or End
// feature prefix, so a direction comes first and the `#` after it.
// KerML 8.2.4.3.1 — FeaturePrefix = ( EndFeaturePrefix OwnedCrossFeatureMember?
//                                   | BasicFeaturePrefix )
//                                   ( ownedRelationship += PrefixMetadataMember )*
// rejects: PARSE-UNEXPECTED on: #M in feature x;
class C {
	#M in feature x;
}
