// REJECTED BY RULE. The `#` is followed by the metaclass it applies: a
// PrefixMetadataFeature is an OwnedFeatureTyping, a qualified name or a feature chain.
// KerML 8.2.5.12  — PrefixMetadataMember  = '#' PrefixMetadataFeature
//                   PrefixMetadataFeature = ownedRelationship += OwnedFeatureTyping
// KerML 8.2.4.1.2 — GeneralType = [QualifiedName] | OwnedFeatureChain
// rejects: PARSE-UNEXPECTED on: # classifier Bicycle;
# classifier Bicycle;
