// REJECTED BY RULE. Neither alternative of FeatureSpecializationPart can produce a
// second MultiplicityPart: the first takes at most one and the second starts with
// the only one it has.
// KerML 8.2.4.3.1 — FeatureSpecializationPart = FeatureSpecialization+
//                       MultiplicityPart? FeatureSpecialization*
//                     | MultiplicityPart FeatureSpecialization*
// `part p[1];` parses, so this is the rule and not an absence.
part p[1][2];
