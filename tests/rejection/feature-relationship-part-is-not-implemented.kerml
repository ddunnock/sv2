// REJECTED BY ABSENCE, not by rule. FeatureDeclaration ends in
// FeatureRelationshipPart*, none of whose four alternatives this parser reads.
// KerML 8.2.4.3.1 — FeatureRelationshipPart = TypeRelationshipPart | ChainingPart
//                                           | InvertingPart | TypeFeaturingPart
// The classifier half of this is type-relationship-part-is-not-implemented.kerml;
// a feature reaches three more parts that a classifier does not, so it gets its own
// case. Remove both when the parts land.
feature f chains a.b;
