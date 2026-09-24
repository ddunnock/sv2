// REJECTED BY RULE. A ConjugationPart names exactly one original type: no comma list,
// unlike the specialization and relationship parts. A Type has "at most one owned
// Conjugation" (KerML 8.3.3.1.10, validateTypeAtMostOneConjugator).
// KerML 8.2.4.1.1 — ConjugationPart = CONJUGATES OwnedConjugation
// rejects: PARSE-EXPECTED on: class B conjugates A, C;
class B conjugates A, C;
