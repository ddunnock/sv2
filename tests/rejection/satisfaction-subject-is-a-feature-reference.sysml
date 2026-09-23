// REJECTED BY RULE. The satisfying feature after `by` is a reference to a feature, a
// qualified name or a feature chain, and not an arbitrary expression.
// SysML 8.2.2.21.2 — SatisfactionSubjectMember = SatisfactionParameter
//                    SatisfactionParameter = SatisfactionFeatureValue
//                    SatisfactionFeatureValue = SatisfactionReferenceExpression
//                    SatisfactionReferenceExpression = FeatureChainMember
// SysML 8.2.2.17.5 — FeatureChainMember = [QualifiedName] | OwnedFeatureChainMember
// rejects: PARSE-EXPECTED on: satisfy vehicleMaximumMass by selectVehicle(fleet);
part def Context {
	satisfy vehicleMaximumMass by selectVehicle(fleet);
}
