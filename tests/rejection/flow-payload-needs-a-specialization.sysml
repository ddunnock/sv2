// REJECTED BY RULE. A payload that names itself and then writes only `ordered` has a
// MultiplicityPart and no FeatureSpecialization. PayloadFeature's first alternative needs
// a PayloadFeatureSpecializationPart, and both of that production's alternatives contain
// at least one FeatureSpecialization; the other two PayloadFeature alternatives take
// their multiplicity as an OwnedMultiplicity, which opens on '[' and never on `ordered`.
// SysML 8.2.2.16  — PayloadFeature = Identification? PayloadFeatureSpecializationPart
//                                    ValuePart?
//                                  | OwnedFeatureTyping OwnedMultiplicity?
//                                  | OwnedMultiplicity OwnedFeatureTyping
//                   PayloadFeatureSpecializationPart =
//                       FeatureSpecialization+ MultiplicityPart? FeatureSpecialization*
//                     | MultiplicityPart FeatureSpecialization+
part def P {
	flow of x ordered from a.b to c.d;
}
