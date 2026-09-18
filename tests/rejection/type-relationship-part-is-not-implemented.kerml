// REJECTED BY ABSENCE, not by rule. ClassifierDeclaration ends in
// TypeRelationshipPart*, none of whose four alternatives this parser reads.
// KerML 8.2.4.2 — TypeRelationshipPart = DisjoiningPart | UnioningPart
//                                      | IntersectingPart | DifferencingPart
// Remove this case when the four parts land.
classifier C unions A, B;
