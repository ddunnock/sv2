// REJECTED BY ABSENCE, not by rule. ClassifierDeclaration's one alternation is
// ( SuperclassingPart | ConjugationPart ); this parser reads the first and not the
// second, so a conjugated classifier is reported rather than accepted.
// KerML 8.2.4.2 — ClassifierDeclaration = 'all'? Identification OwnedMultiplicity?
//                     ( SuperclassingPart | ConjugationPart )? TypeRelationshipPart*
//                 ConjugationPart = ( '~' | 'conjugates' ) OwnedConjugation
// The corpus writes this: vendor/corpus/kerml/src/examples/Simple Tests/Conjugation.kerml
// has `class B conjugates A;`. Remove this case when OwnedConjugation lands.
class B conjugates A;
