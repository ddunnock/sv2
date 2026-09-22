// REJECTED BY RULE. `event` is followed by one of two alternatives, a reference to an
// occurrence or the keyword `occurrence`; neither is optional.
// SysML 8.2.2.9.2 — EventOccurrenceUsage = OccurrenceUsagePrefix 'event'
//                     ( OwnedReferenceSubsetting FeatureSpecializationPart?
//                     | 'occurrence' UsageDeclaration? ) UsageCompletion
// rejects: PARSE-EXPECTED on: event;
part client {
	event;
}
