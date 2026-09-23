// REJECTED BY RULE. An invocation may name its type by a feature chain, and every link of
// the chain is a name: OwnedFeatureChaining is `chainingFeature = [QualifiedName]`, so a
// `.` is always followed by one, and a `(` after a `.` is no link.
// KerML 8.2.5.8.3 — InstantiatedTypeMember = memberElement = InstantiatedTypeReference
//                                          | OwnedFeatureChainMember
// SysML 8.2.2.6.5 — OwnedFeatureChain = OwnedFeatureChaining ( '.' OwnedFeatureChaining )+
// rejects: PARSE-EXPECTED on: out verdict = PassIf(vehicleSpecification.(vehicle_uut));
verification def MassTest {
	out verdict = PassIf(vehicleSpecification.(vehicle_uut));
}
