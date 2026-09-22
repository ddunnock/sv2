// REJECTED BY RULE. An event occurrence usage written with `event` alone refers to the
// occurrence it is: it takes a reference, not a name, before the reference or after it.
// Declaring a name needs the kind keyword `event occurrence` (7.9.5: "the declaration
// does not include either a name or a short name").
// SysML 8.2.2.9.2 — EventOccurrenceUsage = OccurrenceUsagePrefix 'event'
//                     ( OwnedReferenceSubsetting FeatureSpecializationPart?
//                     | 'occurrence' UsageDeclaration? ) UsageCompletion
// rejects: PARSE-EXPECTED on: event subscriptionMessage.source request;
part client {
	event subscriptionMessage.source request;
}
