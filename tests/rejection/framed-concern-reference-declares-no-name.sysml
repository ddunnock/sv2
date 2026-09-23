// REJECTED BY RULE. `frame` alone refers to the concern it frames: "the keyword frame can
// be used rather than frame concern to declare a framed concern using reference
// subsetting" (7.21.3, receipt 0e50a373). A name needs `frame concern`.
// SysML 8.2.2.21.1 — FramedConcernUsage = OwnedReferenceSubsetting
//                      FeatureSpecializationPart? RequirementBody | ...
// rejects: PARSE-EXPECTED on: frame BrakingConcern brakingConcern;
requirement def BrakingRequirement {
	frame BrakingConcern brakingConcern;
}
