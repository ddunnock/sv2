// REJECTED BY RULE. `assert` comes before `not`, both before `satisfy`: the production
// writes them in that order, and deviation SatisfyRequirementUsage (follow_xtext) makes
// each optional without reordering them.
// SysML 8.2.2.21.2 — SatisfyRequirementUsage = OccurrenceUsagePrefix 'assert'
//                      ( isNegated ?= 'not' ) 'satisfy' ...
// rejects: PARSE-UNEXPECTED on: not assert satisfy vehicleMaximumMass by vehicle2;
part def Context {
	part vehicle2 : ExperimentalVehicle;
	not assert satisfy vehicleMaximumMass by vehicle2;
}
