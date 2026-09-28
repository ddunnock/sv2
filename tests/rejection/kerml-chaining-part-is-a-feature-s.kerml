// REJECTED BY RULE. ChainingPart is one of FeatureRelationshipPart's alternatives,
// which only a FeatureDeclaration ends in. A ClassifierDeclaration ends in
// TypeRelationshipPart*, which does not reach it.
// KerML 8.2.4.2.1 — ClassifierDeclaration = 'all'? Identification OwnedMultiplicity?
//                       ( SuperclassingPart | ConjugationPart )? TypeRelationshipPart*
// KerML 8.2.4.3.1 — FeatureRelationshipPart = TypeRelationshipPart | ChainingPart
//                                           | InvertingPart | TypeFeaturingPart
// rejects: PARSE-EXPECTED on: class Cousins chains parents.siblings.children;
class Cousins chains parents.siblings.children;
