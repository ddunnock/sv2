// REJECTED BY RULE. A feature's OwnedSubsetting is a GeneralType: a qualified name or a
// feature chain, and every `.` in a chain is followed by a link.
// KerML 8.2.4.3.3 — OwnedSubsetting = GeneralType
// KerML 8.2.4.1.2 — GeneralType     = [QualifiedName] | OwnedFeatureChain
// KerML 8.2.4.3.5 — FeatureChain    = OwnedFeatureChaining ( '.' OwnedFeatureChaining )+
// rejects: PARSE-EXPECTED on: feature driveWheels subsets vehicle.;
struct Automobile {
	feature driveWheels subsets vehicle.;
}
