// REJECTED BY RULE. A variant written without a kind keyword is a VariantReference, which
// names a separately declared usage and ends in a UsageBody: it has no ValuePart, where a
// DefaultReferenceUsage ends in a UsageCompletion that has one.
// SysML 8.2.2.6.3 — VariantReference = OwnedReferenceSubsetting FeatureSpecialization*
//                                      UsageBody
// rejects: PARSE-EXPECTED on: variant x = 1;
variation part def P {
	variant x = 1;
}
