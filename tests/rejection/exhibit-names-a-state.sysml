// REJECTED BY RULE. `exhibit` is followed by `state` and a declaration, or by the
// qualified name of the state exhibited: "the exhibited state of the exhibit state usage
// is identified by giving a qualified name or feature chain immediately after the exhibit
// keyword" (7.18.4, receipt dbedb2cc). It exhibits no other kind of usage.
// SysML 8.2.2.18.2 — ExhibitStateUsage = OccurrenceUsagePrefix 'exhibit'
//                      ( OwnedReferenceSubsetting FeatureSpecializationPart?
//                      | 'state' UsageDeclaration ) ValuePart? StateUsageBody
// rejects: PARSE-EXPECTED on: exhibit part p;
part def P {
	exhibit part p;
}
