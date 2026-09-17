// REJECTION BY ABSENCE, not by rule. `part frontSeat[2];` is well-formed SysML and
// the pinned corpus writes it nine times. MultiplicityPart reaches OwnedMultiplicity
// and so an expression parser, which sv2-syntax does not have yet.
// KerML 8.2.4.3.1 — FeatureSpecializationPart = FeatureSpecialization+
//                       MultiplicityPart? FeatureSpecialization*
//                     | MultiplicityPart FeatureSpecialization*
// KerML 8.2.4.3.1 — MultiplicityPart = OwnedMultiplicity
//                     | OwnedMultiplicity? ( 'ordered' 'nonunique'? | 'nonunique' 'ordered'? )
// Remove this case when MultiplicityPart lands.
part frontSeat[2];
