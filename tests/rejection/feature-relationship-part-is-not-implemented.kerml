// REJECTED BY ABSENCE, not by rule. FeatureDeclaration ends in
// FeatureRelationshipPart*, and this parser does not read ChainingPart, one of its
// four alternatives.
// KerML 8.2.4.3.1 — FeatureRelationshipPart = TypeRelationshipPart | ChainingPart
//                                           | InvertingPart | TypeFeaturingPart
// TypeRelationshipPart and TypeFeaturingPart are read; ChainingPart and InvertingPart
// are not. Remove this case when ChainingPart lands.
feature f chains a.b;
