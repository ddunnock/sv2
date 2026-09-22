// REJECTED BY RULE. VariantUsageElement replaces DefaultReferenceUsage with
// VariantReference, which opens on the reference to the usage it varies: a variant
// declares no name of its own, so a bare specialization with nothing before it is no
// variant, although `:>> x;` alone is a DefaultReferenceUsage.
// SysML 8.2.2.6.4 — VariantUsageElement = VariantReference | ReferenceUsage | …
// SysML 8.2.2.6.3 — VariantReference = OwnedReferenceSubsetting FeatureSpecialization*
//                                      UsageBody
// rejects: PARSE-EXPECTED on: variant :>> x;
variation part def P {
	variant :>> x;
}
