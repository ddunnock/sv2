// REJECTED BY RULE. The keyword for "introduces a loop variable declaration" (7.17.12,
// receipt b0446148): a UsageDeclaration, whose Identification is a name and not a
// feature chain naming an existing feature.
// SysML 8.2.2.17.7 — ForVariableDeclaration = UsageDeclaration
// SysML 8.2.2.6.2  — UsageDeclaration = Identification FeatureSpecializationPart?
// rejects: PARSE-EXPECTED on: for vehicle.power in powerProfile {
action def Drive {
	for vehicle.power in powerProfile {
		assign position := ComputeDynamics(position, vehicle.power);
	}
}
