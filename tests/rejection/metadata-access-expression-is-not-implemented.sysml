// REJECTED BY ABSENCE, not by rule. `E.metadata` is well-formed SysML.
// KerML 8.2.5.8.3 — MetadataAccessExpression =
//                     ownedRelationship += ElementReferenceMember '.' 'metadata'
// This case exists because FeatureChainExpression now reads a postfix '.', and the two
// productions share every token before the name after it. `metadata` is a pinned
// keyword and a keyword is not a name (KerML 8.2.2.6), so at_feature_chain declines it
// and it is reported rather than quietly read as a feature called `metadata`. Without
// this file, implementing the chain would have silently swallowed a different
// production. Removing this rejection means implementing MetadataAccessExpression.
attribute x = E.metadata;
