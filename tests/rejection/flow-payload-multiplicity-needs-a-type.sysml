// REJECTED BY RULE. A payload that opens on a multiplicity is either the third
// PayloadFeature alternative, which must go on to a type, or the first, whose
// PayloadFeatureSpecializationPart must go on to a FeatureSpecialization. `of [1]`
// followed by `from` is neither.
// SysML 8.2.2.16  — PayloadFeature = Identification? PayloadFeatureSpecializationPart
//                                    ValuePart?
//                                  | OwnedFeatureTyping OwnedMultiplicity?
//                                  | OwnedMultiplicity OwnedFeatureTyping
//                   PayloadFeatureSpecializationPart =
//                       FeatureSpecialization+ MultiplicityPart? FeatureSpecialization*
//                     | MultiplicityPart FeatureSpecialization+
part def P {
	flow of [1] from a.b to c.d;
}
