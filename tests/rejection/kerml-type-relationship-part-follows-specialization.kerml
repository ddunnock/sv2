// REJECTED BY RULE. The relationship parts end a declaration: they follow the
// SuperclassingPart, never precede it.
// KerML 8.2.4.2.1 — ClassifierDeclaration = 'all'? Identification OwnedMultiplicity?
//                       ( SuperclassingPart | ConjugationPart )? TypeRelationshipPart*
// rejects: PARSE-EXPECTED on: classifier E intersects A, B specializes C;
classifier E intersects A, B specializes C;
