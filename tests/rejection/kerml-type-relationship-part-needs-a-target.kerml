// REJECTED BY RULE. Each TypeRelationshipPart names at least one type after its keyword,
// and each comma one more: the list is not optional and does not end on a comma.
// KerML 8.2.4.1.1 — UnioningPart = 'unions' Unioning ( ',' Unioning )*
//                   Unioning     = [QualifiedName] | OwnedFeatureChain   (8.2.4.1.5)
// rejects: PARSE-EXPECTED on: classifier OurBicycle unions;
classifier OurBicycle unions;
