// REJECTED BY RULE. A flow's source end after `from` is paired with a target end after
// `to`; one end alone is no FlowDeclaration.
// KerML 8.2.5.9.2 — FlowDeclaration = FeatureDeclaration ValuePart?
//                       ( 'of' PayloadFeatureMember )?
//                       ( 'from' FlowEndMember 'to' FlowEndMember )?
//                   | ( 'all' )? FlowEndMember 'to' FlowEndMember
// rejects: PARSE-EXPECTED on: flow fuelFlow from fuelTank.fuelOut;
struct Vehicle {
	flow fuelFlow from fuelTank.fuelOut;
}
