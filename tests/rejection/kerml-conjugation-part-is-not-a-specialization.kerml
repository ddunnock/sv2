// REJECTED BY RULE. A declaration writes a specialization or a conjugation, never both:
// a conjugated type "may not also be the specific Type in any Specialization" (KerML
// 8.3.3.1.2, receipt eabb0d9b), and the grammar says so by alternation.
// KerML 8.2.4.2.1 — ClassifierDeclaration = 'all'? Identification OwnedMultiplicity?
//                       ( SuperclassingPart | ConjugationPart )? TypeRelationshipPart*
// rejects: PARSE-EXPECTED on: class B specializes A conjugates C;
class B specializes A conjugates C;
