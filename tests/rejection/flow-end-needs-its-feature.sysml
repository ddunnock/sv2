// REJECTED BY RULE. A FlowEnd always ends in its FlowFeatureMember; the FlowEndSubsetting
// before it, with its '.', is the optional part. `a.` is a subsetting with no feature.
// SysML 8.2.2.16  — FlowEnd = ( FlowEndSubsetting )? FlowFeatureMember
//                   FlowEndSubsetting = [QualifiedName] '.' | FeatureChainPrefix
//                   (the '.' is deviation FlowEndSubsetting, follow_xtext)
//                   FlowFeatureMember = FlowFeature ; FlowFeature = FlowFeatureRedefinition
//                   FlowFeatureRedefinition = [QualifiedName]
part def P {
	flow a. to c.d;
}
